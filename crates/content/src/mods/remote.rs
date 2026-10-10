// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Online mod metadata lookup with a persistent on-disk cache.
//!
//! Four JSON files live under [`storage::LOCATIONS`]'s cache directory, plus a
//! directory of icons beside them:
//!
//! - `modrinth.json` and `curseforge.json` cache the result of each platform's
//!   file-feature lookup, keyed by the file's SHA-512 checksum. Entries expire
//!   after `REMOTE_CACHE_TTL_SECS` and only carry list-display fields.
//! - `local.json` caches the local parse result so parsing is skipped on the
//!   next run. It never expires.
//! - `identity.json` maps each file's SHA-512 checksum to the project/mod ids
//!   it is known on either platform. It never expires and only stores the
//!   platform ids, not the lookup details.
//! - `local-icons/<sha512>.png` is one file's own icon, named after the same
//!   checksum. The icon used to sit in `local.json` as a `data:` URL, which made
//!   it the bulk of that file — and nearly all of it was thrown away, because
//!   `merge_remote` swaps a local icon for the platform's own URL whenever the
//!   lookup matched, which is the common case. Only a mod neither platform knows
//!   reads its icon back at all, so the bytes sit beside the JSON rather than in
//!   it: raw instead of base64, and read one file at a time instead of
//!   inflating every parse with the whole set.
//!
//! The platform lookups for a file are classified by its identity entry:
//!
//! - When the identity carries both ids, the lookup follows Modrinth first
//!   (faster), then CurseForge — cache first, then a fresh query.
//! - Otherwise both platforms are queried at the same time. The first response
//!   wins for the frontend result, but both responses are fully processed and
//!   written to the caches. A platform that failed or has no data for the file
//!   records `null` for its id in the identity entry.
//!
//! Online info takes priority over the local parse result; local data
//! supplements whatever the platform does not provide. Icons found online are
//! passed through as URLs. An embedded (jar-in-jar) mod is a different mod that
//! happens to ship inside the same file, so it keeps its own name, version and
//! icon instead of taking the outer jar's.
//!
//! # Writing
//!
//! The files are shared between concurrent parse requests — the frontend
//! switching instances mid-parse is enough to start two — so every write is a
//! read-merge-write behind `CACHE_WRITE_LOCK` that re-reads the current
//! on-disk state before merging, and two parses never overwrite each other's
//! entries. Reads take no lock: one that races a concurrent write merely misses
//! that entry, and the merge re-reads the file under the lock regardless, so
//! locking the read path would only add contention to folder parsing for nothing.
//!
//! `merge_and_save` is the only place the lock is taken, and that is what keeps
//! it deadlock-free. Three invariants hold it together:
//!
//! 1. One acquisition, never nested. A `tokio::sync::Mutex` is not reentrant, so
//!    the helper calls only `load_cache` and `save_cache`, which take no lock
//!    of their own — no cycle is reachable.
//! 2. Nothing but disk I/O and serde runs under the lock. No `.await` there
//!    reaches back into this module; every platform query happens before the
//!    saves.
//! 3. The synchronous hashing of a folder (`sha512_file`) runs outside the
//!    lock, before anything is saved.

use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use futures::{FutureExt, select};
use log::warn;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha512};

use crate::error::Result;
use crate::mods::{
    ModLoader, ResolvedAuthorInfo, ResolvedMod, decode_icon_data_url, encode_icon,
    is_disabled_file, parse_mod,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteModPlatform {
    Modrinth,
    CurseForge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteModInfo {
    pub platform: RemoteModPlatform,
    pub project_id: String,
    pub version_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub authors: Vec<String>,
    pub download_url: Option<String>,
    pub version_number: Option<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

pub(crate) fn sha512_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

const REMOTE_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

const MODRINTH_CACHE: &str = "modrinth.json";
const CURSEFORGE_CACHE: &str = "curseforge.json";
const LOCAL_CACHE: &str = "local.json";
const IDENTITY_CACHE: &str = "identity.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteCacheEntry {
    #[serde(rename = "ts")]
    timestamp: u64,
    #[serde(flatten)]
    info: RemoteModInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalCacheEntry {
    mods: Vec<ResolvedMod>,
}

/// The platform ids a file is known on. A missing or failed lookup is `None`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct IdentityEntry {
    #[serde(default)]
    modrinth: Option<String>,
    #[serde(default)]
    curseforge: Option<String>,
}

type RemoteCache = HashMap<String, RemoteCacheEntry>;
type LocalCache = HashMap<String, LocalCacheEntry>;
type IdentityCache = HashMap<String, IdentityEntry>;

/// Serializes the read-merge-write of the shared cache files so concurrent
/// parse requests (e.g. the frontend switching instances mid-parse) never
/// overwrite each other's entries. See the module docs for the invariants that
/// keep taking it deadlock-free.
static CACHE_WRITE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

async fn cache_write_lock() -> tokio::sync::MutexGuard<'static, ()> {
    CACHE_WRITE_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn cache_dir() -> std::path::PathBuf {
    storage::LOCATIONS.launcher.cache.join("mods")
}

/// The icon of the file with this checksum, stored beside the JSON caches. Named
/// after the content, so a replaced jar simply gets a new name and nothing has
/// to notice that the old one is now stale.
fn icon_path(hash: &str) -> PathBuf {
    cache_dir().join("local-icons").join(format!("{hash}.png"))
}

async fn load_cache<T: DeserializeOwned>(name: &str) -> HashMap<String, T> {
    let path = cache_dir().join(name);
    match tokio::fs::read(&path).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(cache) => cache,
            Err(error) => {
                warn!(
                    "{} could not be read as a mod cache ({error}); it will be rebuilt",
                    path.display()
                );
                HashMap::default()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::default(),
        Err(error) => {
            warn!("Could not read {}: {error}", path.display());
            HashMap::default()
        }
    }
}

async fn save_cache<T: Serialize>(name: &str, cache: &HashMap<String, T>) {
    let Ok(bytes) = serde_json::to_vec(cache) else {
        warn!("Failed to serialize mod cache {name}");
        return;
    };
    if let Err(error) = tokio::fs::write(cache_dir().join(name), bytes).await {
        warn!("Failed to save mod cache {name}: {error}");
    }
}

/// Merge the given entries into the named cache and write it back.
///
/// The current on-disk state is re-read inside `CACHE_WRITE_LOCK` so entries
/// written by a concurrent parse are kept. This is the only place the lock is
/// taken — see the module docs.
async fn merge_and_save<T>(name: &str, updates: &HashMap<String, T>)
where
    T: Clone + Serialize + DeserializeOwned,
{
    if updates.is_empty() {
        return;
    }
    let _guard = cache_write_lock().await;
    let mut cache = load_cache(name).await;
    for (hash, entry) in updates {
        cache.insert(hash.clone(), entry.clone());
    }
    save_cache(name, &cache).await;
}

async fn load_identity_cache() -> IdentityCache {
    load_cache(IDENTITY_CACHE).await
}

async fn save_identity_cache(cache: &IdentityCache) {
    save_cache(IDENTITY_CACHE, cache).await;
}

/// Merge the per-platform lookup outcomes (hash → `Some(id)` on success,
/// `None` on failure) into the identity cache field-wise and write it back.
/// `None` only overwrites the platform it belongs to, so two concurrent parses
/// discovering the same file on different platforms do not lose each other's id.
///
/// The merge is field-wise rather than entry-wise because of that, which is why
/// it does not go through `merge_and_save` like the other three caches: there
/// the entry *is* the value, and here only one of its two fields is.
async fn merge_and_save_identity(
    modrinth: HashMap<String, Option<String>>,
    curseforge: HashMap<String, Option<String>>,
) {
    if modrinth.is_empty() && curseforge.is_empty() {
        return;
    }
    let _guard = cache_write_lock().await;
    let mut cache = load_identity_cache().await;
    for (hash, id) in modrinth {
        cache.entry(hash).or_default().modrinth = id;
    }
    for (hash, id) in curseforge {
        cache.entry(hash).or_default().curseforge = id;
    }
    save_identity_cache(&cache).await;
}

fn cache_is_fresh(entry: &RemoteCacheEntry, timestamp: u64) -> bool {
    timestamp.saturating_sub(entry.timestamp) < REMOTE_CACHE_TTL_SECS
}

pub async fn parse_folder_with_remote<S: AsRef<Path> + ?Sized>(folder: &S) -> Vec<ResolvedMod> {
    let folder = folder.as_ref();
    let files: Vec<PathBuf> = folder
        .read_dir()
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect()
        })
        .unwrap_or_default();

    if let Err(error) = tokio::fs::create_dir_all(cache_dir()).await {
        warn!("Failed to create mod cache directory: {error}");
        return Vec::new();
    }

    let local_cache = load_cache::<LocalCacheEntry>(LOCAL_CACHE).await;
    let timestamp = now();
    // One entry per file, so files that share the same content (and therefore
    // the same cache key) still show up individually with their own `disabled`
    // flag.
    let mut files_with_mods: Vec<(String, PathBuf, Vec<ResolvedMod>)> = Vec::new();
    let mut files_by_hash: HashMap<String, PathBuf> = HashMap::new();
    // Only what this parse produced. `local_cache` is whatever is on disk, and
    // the merge re-reads it under the lock, so passing the whole map would
    // rewrite every entry to write the few that are new.
    let mut new_local_entries: LocalCache = HashMap::new();

    for path in files {
        let hash = match sha512_file(&path) {
            Ok(hash) => hash,
            Err(error) => {
                warn!("Failed to hash {:?}: {error}", path);
                continue;
            }
        };
        let cached = local_cache.get(&hash).cloned();
        let mut parsed = match cached {
            Some(entry) => entry.mods,
            None => match parse_mod(&path) {
                Ok(mods) => mods,
                Err(crate::error::Error::NotAModFile) => continue,
                Err(error) => {
                    warn!("Failed to parse mod {:?}: {error}", path);
                    vec![ResolvedMod::unrecognized(&path)]
                }
            },
        };
        if parsed.is_empty() {
            continue;
        }
        // An entry is written when it is new, or when the cached copy still
        // carries an icon inline. The second case is the migration: the icon
        // moves to its own file and the JSON is rewritten without it. Both stop
        // repeating on the next parse, so each entry costs one rewrite ever —
        // without it a cache written before the split would keep its megabytes
        // of base64 forever, since nothing else ever invalidates a valid entry.
        if !local_cache.contains_key(&hash) || own_icon(&parsed).is_some() {
            new_local_entries.insert(
                hash.clone(),
                LocalCacheEntry {
                    mods: parsed.clone(),
                },
            );
        }
        // `disabled` is a property of the file name, not the content, so it
        // must be derived from the current file even when the parse result
        // came from the cache. The same goes for `path`, which the cached
        // entries may carry from an earlier run.
        let disabled = is_disabled_file(&path);
        for mod_info in &mut parsed {
            mod_info.disabled = disabled;
            mod_info.path = path.clone();
        }
        files_by_hash.insert(hash.clone(), path.clone());
        files_with_mods.push((hash, path, parsed));
    }

    // The icons go out before the JSON, because they come off the same entries:
    // once `merge_and_save` writes, this parse's mods are on disk without them.
    write_icons(&new_local_entries).await;
    merge_and_save(LOCAL_CACHE, &new_local_entries).await;

    let mut modrinth_cache = load_cache::<RemoteCacheEntry>(MODRINTH_CACHE).await;
    let mut curseforge_cache = load_cache::<RemoteCacheEntry>(CURSEFORGE_CACHE).await;
    let identity_cache = load_identity_cache().await;

    // Classify every hash by its identity: hashes known on both platforms take
    // the sequential Modrinth-then-CurseForge path, everything else is queried
    // on both platforms at once.
    let mut group_a: Vec<String> = Vec::new();
    let mut group_b: Vec<String> = Vec::new();
    for (hash, ..) in &files_with_mods {
        match identity_cache.get(hash) {
            Some(identity) if identity.modrinth.is_some() && identity.curseforge.is_some() => {
                group_a.push(hash.clone());
            }
            _ => group_b.push(hash.clone()),
        }
    }

    let mut remote_by_hash: HashMap<String, RemoteModInfo> = HashMap::new();
    let mut modrinth_identity: HashMap<String, Option<String>> = HashMap::new();
    let mut curseforge_identity: HashMap<String, Option<String>> = HashMap::new();
    let mut modrinth_dirty = false;
    let mut curseforge_dirty = false;

    // Group A: both platform ids known. Each platform's cache is consulted
    // first; whatever is still missing is queried from Modrinth, then
    // CurseForge.
    let mut needs_modrinth: Vec<String> = Vec::new();
    for hash in &group_a {
        if let Some(entry) = modrinth_cache.get(hash)
            && cache_is_fresh(entry, timestamp)
        {
            remote_by_hash.insert(hash.clone(), entry.info.clone());
        } else if let Some(entry) = curseforge_cache.get(hash)
            && cache_is_fresh(entry, timestamp)
        {
            remote_by_hash.insert(hash.clone(), entry.info.clone());
        } else {
            needs_modrinth.push(hash.clone());
        }
    }
    let cached = remote_by_hash.len();
    log::debug!(
        "Resolving remote info for {} mod(s): {} answered from cache, {} to query on \
         Modrinth",
        group_a.len() + group_b.len(),
        cached,
        needs_modrinth.len()
    );
    if !needs_modrinth.is_empty() {
        let (info, _) = query_modrinth_batch(&needs_modrinth, &mut modrinth_cache).await;
        log::debug!("Modrinth answered with {} of them", info.len());
        for (hash, remote) in info {
            remote_by_hash.entry(hash).or_insert(remote);
        }
        modrinth_dirty = true;
    }
    let needs_curseforge: Vec<String> = group_a
        .iter()
        .filter(|hash| !remote_by_hash.contains_key(*hash))
        .cloned()
        .collect();
    if !needs_curseforge.is_empty() {
        let (info, _) =
            query_curseforge_batch(&needs_curseforge, &files_by_hash, &mut curseforge_cache).await;
        for (hash, remote) in info {
            remote_by_hash.entry(hash).or_insert(remote);
        }
        curseforge_dirty = true;
    }

    // Group B: identity missing or incomplete. Both platforms are queried
    // concurrently; the first response wins for the frontend, while the slower
    // platform is still fully processed and written to the caches.
    if !group_b.is_empty() {
        // Fresh cache entries are used without re-querying. Cached Modrinth
        // data wins when both platforms already have an entry.
        let mut needs_modrinth_b: Vec<String> = Vec::new();
        let mut needs_curseforge_b: Vec<String> = Vec::new();
        for hash in &group_b {
            if let Some(entry) = modrinth_cache.get(hash)
                && cache_is_fresh(entry, timestamp)
            {
                remote_by_hash
                    .entry(hash.clone())
                    .or_insert(entry.info.clone());
                modrinth_identity.insert(hash.clone(), Some(entry.info.project_id.clone()));
            } else {
                needs_modrinth_b.push(hash.clone());
            }
            if let Some(entry) = curseforge_cache.get(hash)
                && cache_is_fresh(entry, timestamp)
            {
                remote_by_hash
                    .entry(hash.clone())
                    .or_insert(entry.info.clone());
                curseforge_identity.insert(hash.clone(), Some(entry.info.project_id.clone()));
            } else {
                needs_curseforge_b.push(hash.clone());
            }
        }

        let mut modrinth_fut =
            Box::pin(query_modrinth_batch(&needs_modrinth_b, &mut modrinth_cache)).fuse();
        let mut curseforge_fut = Box::pin(query_curseforge_batch(
            &needs_curseforge_b,
            &files_by_hash,
            &mut curseforge_cache,
        ))
        .fuse();
        let mut done = 0;
        while done < 2 {
            select! {
                result = modrinth_fut => {
                    let (info, identity) = result;
                    for (hash, remote) in info {
                        remote_by_hash.entry(hash).or_insert(remote);
                    }
                    modrinth_identity.extend(identity);
                }
                result = curseforge_fut => {
                    let (info, identity) = result;
                    for (hash, remote) in info {
                        remote_by_hash.entry(hash).or_insert(remote);
                    }
                    curseforge_identity.extend(identity);
                }
            }
            done += 1;
        }
        modrinth_dirty = true;
        curseforge_dirty = true;
    }

    if modrinth_dirty {
        merge_and_save(MODRINTH_CACHE, &modrinth_cache).await;
    }
    if curseforge_dirty {
        merge_and_save(CURSEFORGE_CACHE, &curseforge_cache).await;
    }
    merge_and_save_identity(modrinth_identity, curseforge_identity).await;

    // Merge online info into the local parse results. Multiple files may share
    // the same hash (identical content), so the entry is kept for the rest.
    let mut result = Vec::new();
    for (hash, _, mut mods) in files_with_mods {
        let remote = remote_by_hash.get(&hash);
        // The platform's icon wins wherever there is one, so the file beside the
        // caches is read only for a mod neither platform knows. A `None` icon on
        // a mod that came from `local.json` says exactly that — the cache no
        // longer carries icons, they live in that file now.
        let local_icon = if remote
            .is_none_or(|remote| remote.icon_url.as_deref().unwrap_or_default().is_empty())
        {
            read_icon_file(&hash).await
        } else {
            None
        };
        for mod_info in &mut mods {
            // An embedded mod is a different mod that happens to ship inside this
            // file. It has its own name, version and icon, and the file's sidecar
            // icon is the outer jar's — so neither the online info nor that icon
            // is applied to it. Skipped rather than merged-and-patched because
            // there is nothing to merge from.
            if mod_info.embedded {
                continue;
            }
            if mod_info.icon.is_none() {
                mod_info.icon = local_icon.clone();
            }
            if let Some(remote) = remote {
                merge_remote(mod_info, remote);
            }
        }
        result.extend(mods);
    }
    result
}

/// Writes a newly parsed file's icon beside the JSON caches.
///
/// One icon per file, named after its checksum — the first mod the file declares.
/// Every loader that bundles jars pushes the file's own metadata before them, so
/// that is the file's own icon; a `mods.toml` may declare several mods of its own,
/// and the rest go without, because they share the file's checksum and so its
/// online lookup, whose icon covers them anyway.
async fn write_icons(entries: &LocalCache) {
    if entries.is_empty() {
        return;
    }
    if let Err(error) = tokio::fs::create_dir_all(cache_dir().join("local-icons")).await {
        warn!("Failed to create the mod icon cache directory: {error}");
        return;
    }
    for (hash, entry) in entries {
        let Some(bytes) = own_icon(&entry.mods).and_then(decode_icon_data_url) else {
            continue;
        };
        if let Err(error) = tokio::fs::write(icon_path(hash), bytes).await {
            warn!("Failed to save the cached icon of {hash}: {error}");
        }
    }
}

/// The icon [`write_icons`] stores for these mods: the first one the file declares
/// rather than embeds.
///
/// The caller uses this too, to tell an entry that still has an icon to migrate
/// from one that has none left. They have to agree on it — an entry whose icon
/// this does not find would be rewritten on every parse forever, since the JSON
/// never loses the icons that were already in it.
fn own_icon(mods: &[ResolvedMod]) -> Option<&str> {
    mods.iter()
        .find(|mod_info| !mod_info.embedded)
        .and_then(|mod_info| mod_info.icon.as_deref())
}

async fn read_icon_file(hash: &str) -> Option<String> {
    let bytes = tokio::fs::read(icon_path(hash)).await.ok()?;
    Some(encode_icon(bytes))
}

/// Query Modrinth for a batch of SHA-512 hashes and store the fresh results
/// in the cache. Runs until the first request error, which only logs a warning.
///
/// Returns the resolved mods keyed by hash and, for every requested hash, the
/// Modrinth project id (`Some`) or `None` when the lookup failed or found no
/// data.
async fn query_modrinth_batch(
    hashes: &[String],
    cache: &mut RemoteCache,
) -> (
    HashMap<String, RemoteModInfo>,
    HashMap<String, Option<String>>,
) {
    let mut identity: HashMap<String, Option<String>> =
        hashes.iter().map(|hash| (hash.clone(), None)).collect();
    if hashes.is_empty() {
        return (HashMap::new(), identity);
    }
    let versions = match modrinth::get_versions_from_hashes(hashes, "sha512").await {
        Ok(versions) => versions,
        Err(error) => {
            warn!("Failed to look up mods on Modrinth: {error}");
            return (HashMap::new(), identity);
        }
    };
    if versions.is_empty() {
        return (HashMap::new(), identity);
    }

    let project_ids: Vec<String> = versions
        .values()
        .filter_map(|version| version.get("project_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    let id_refs: Vec<&str> = project_ids.iter().map(String::as_str).collect();
    let projects = match modrinth::get_projects(&id_refs).await {
        Ok(projects) => projects,
        Err(error) => {
            warn!("Failed to fetch Modrinth projects: {error}");
            return (HashMap::new(), identity);
        }
    };
    let projects: HashMap<String, Value> = projects
        .as_array()
        .map(|projects| {
            projects
                .iter()
                .filter_map(|project| {
                    let id = project.get("id").and_then(Value::as_str)?;
                    Some((id.to_string(), project.clone()))
                })
                .collect()
        })
        .unwrap_or_default();

    let timestamp = now();
    let mut info_by_hash: HashMap<String, RemoteModInfo> = HashMap::new();
    for (hash, version) in &versions {
        let Some(version) = version.as_object() else {
            continue;
        };
        let Some(project_id) = version.get("project_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(project) = projects.get(project_id) else {
            continue;
        };
        let Some(project) = project.as_object() else {
            continue;
        };
        let Some(team) = project.get("team").and_then(Value::as_str) else {
            continue;
        };
        // Team members are not served by the mirror, so fetch them from the
        // official API on a best-effort basis.
        let members = modrinth::get_team_members(team).await;
        let authors = match members {
            Ok(members) => members
                .as_array()
                .map(|members| {
                    members
                        .iter()
                        .filter_map(|member| member.pointer("/user/username"))
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            Err(error) => {
                warn!("Failed to fetch Modrinth team members: {error}");
                Vec::new()
            }
        };
        let info = RemoteModInfo {
            platform: RemoteModPlatform::Modrinth,
            project_id: project_id.to_string(),
            version_id: version
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string),
            name: project
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            description: project
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            icon_url: project
                .get("icon_url")
                .and_then(Value::as_str)
                .map(str::to_string),
            authors,
            download_url: version
                .get("files")
                .and_then(Value::as_array)
                .and_then(|files| files.first())
                .and_then(|file| file.get("url"))
                .and_then(Value::as_str)
                .map(str::to_string),
            version_number: version
                .get("version_number")
                .and_then(Value::as_str)
                .map(str::to_string),
            loaders: version
                .get("loaders")
                .and_then(Value::as_array)
                .map(|loaders| {
                    loaders
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            game_versions: version
                .get("game_versions")
                .and_then(Value::as_array)
                .map(|versions| {
                    versions
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        };
        cache.insert(
            hash.clone(),
            RemoteCacheEntry {
                timestamp,
                info: info.clone(),
            },
        );
        info_by_hash.insert(hash.clone(), info.clone());
        identity.insert(hash.clone(), Some(project_id.to_string()));
    }
    (info_by_hash, identity)
}

/// Query CurseForge for a batch of fingerprints and store the fresh results
/// in the cache. Runs until the first request error, which only logs a warning.
///
/// Returns the resolved mods keyed by hash and, for every requested hash, the
/// CurseForge mod id (`Some`) or `None` when the lookup failed or found no data.
async fn query_curseforge_batch(
    hashes: &[String],
    files_by_hash: &HashMap<String, PathBuf>,
    cache: &mut RemoteCache,
) -> (
    HashMap<String, RemoteModInfo>,
    HashMap<String, Option<String>>,
) {
    let mut identity: HashMap<String, Option<String>> =
        hashes.iter().map(|hash| (hash.clone(), None)).collect();
    if hashes.is_empty() {
        return (HashMap::new(), identity);
    }
    // CurseForge indexes files by fingerprint, so fingerprints must be computed
    // from the real files on disk first.
    let mut fingerprint_of: HashMap<u32, String> = HashMap::new();
    for hash in hashes {
        let path = files_by_hash.get(hash);
        let fingerprint = match path {
            Some(path) => curseforge::compute_fingerprint(path),
            None => continue,
        };
        let fingerprint = match fingerprint {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                warn!("Failed to fingerprint {:?}: {error}", path);
                continue;
            }
        };
        fingerprint_of.insert(fingerprint, hash.clone());
    }

    if fingerprint_of.is_empty() {
        return (HashMap::new(), identity);
    }

    let fingerprints: Vec<u32> = fingerprint_of.keys().copied().collect();
    let value =
        match curseforge::get_fingerprint_matches(curseforge::MINECRAFT_GAME_ID, &fingerprints)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                warn!("Failed to look up mods on CurseForge: {error}");
                return (HashMap::new(), identity);
            }
        };
    let Some(exact_matches) = value
        .pointer("/data/exactMatches")
        .and_then(serde_json::Value::as_array)
    else {
        return (HashMap::new(), identity);
    };

    let timestamp = now();
    let mut info_by_hash: HashMap<String, RemoteModInfo> = HashMap::new();
    for exact in exact_matches {
        let Some(file) = exact.get("file") else {
            continue;
        };
        let Some(fingerprint) = file
            .get("fileFingerprint")
            .and_then(serde_json::Value::as_u64)
            .map(|id| id as u32)
        else {
            continue;
        };
        let Some(hash) = fingerprint_of.get(&fingerprint).cloned() else {
            continue;
        };
        let Some(mod_id) = file.get("modId").and_then(serde_json::Value::as_i64) else {
            continue;
        };
        let mod_value = match curseforge::get_mod(mod_id).await {
            Ok(value) => value,
            Err(error) => {
                warn!("Failed to fetch CurseForge mod {mod_id}: {error}");
                continue;
            }
        };
        let mod_data = &mod_value["data"];
        let authors = mod_data
            .get("authors")
            .and_then(serde_json::Value::as_array)
            .map(|authors| {
                authors
                    .iter()
                    .filter_map(|author| author.get("name").and_then(serde_json::Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let game_versions = file
            .get("gameVersions")
            .and_then(serde_json::Value::as_array)
            .map(|versions| {
                versions
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let info = RemoteModInfo {
            platform: RemoteModPlatform::CurseForge,
            project_id: mod_id.to_string(),
            version_id: file
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .map(|id| id.to_string()),
            name: mod_data
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            description: mod_data
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            icon_url: mod_data
                .pointer("/logo/url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            authors,
            download_url: file
                .get("downloadUrl")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            version_number: file
                .get("displayName")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            loaders: Vec::new(),
            game_versions,
        };
        cache.insert(
            hash.clone(),
            RemoteCacheEntry {
                timestamp,
                info: info.clone(),
            },
        );
        info_by_hash.insert(hash.clone(), info.clone());
        identity.insert(hash.clone(), Some(mod_id.to_string()));
    }
    (info_by_hash, identity)
}

/// Resolve one file's metadata from a single platform only, checking that
/// platform's cache first and querying it when the cache is missing or stale.
/// Returns the remote info (if any) and the identity update for the platform.
async fn resolve_platform_single(
    hash: &str,
    platform: RemoteModPlatform,
    modrinth_cache: &mut RemoteCache,
    curseforge_cache: &mut RemoteCache,
    files_by_hash: &HashMap<String, PathBuf>,
) -> (Option<RemoteModInfo>, HashMap<String, Option<String>>) {
    let timestamp = now();
    match platform {
        RemoteModPlatform::Modrinth => {
            if let Some(entry) = modrinth_cache.get(hash)
                && cache_is_fresh(entry, timestamp)
            {
                let mut identity = HashMap::new();
                identity.insert(hash.to_string(), Some(entry.info.project_id.clone()));
                (Some(entry.info.clone()), identity)
            } else {
                let (info, identity) =
                    query_modrinth_batch(&[hash.to_string()], modrinth_cache).await;
                (info.into_iter().next().map(|(_, info)| info), identity)
            }
        }
        RemoteModPlatform::CurseForge => {
            if let Some(entry) = curseforge_cache.get(hash)
                && cache_is_fresh(entry, timestamp)
            {
                let mut identity = HashMap::new();
                identity.insert(hash.to_string(), Some(entry.info.project_id.clone()));
                (Some(entry.info.clone()), identity)
            } else {
                let (info, identity) =
                    query_curseforge_batch(&[hash.to_string()], files_by_hash, curseforge_cache)
                        .await;
                (info.into_iter().next().map(|(_, info)| info), identity)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInstalledInfo {
    pub installed: bool,
    /// The parsed mods of the installed file(s) whose platform id matches.
    /// Empty when `installed` is `false`.
    pub mods: Vec<ResolvedMod>,
}

/// Check whether a mod identified by its id on a platform is installed in an
/// instance's mods folder.
///
/// The identity cache maps file hashes to platform ids, so a matching hash can
/// be found without querying any API. The matching files are then parsed and
/// resolved from the requested platform only (its cache, then a fresh query).
pub async fn check_mod_installed(
    instance_id: &str,
    platform: RemoteModPlatform,
    project_id: &str,
) -> ModInstalledInfo {
    let empty = ModInstalledInfo {
        installed: false,
        mods: Vec::new(),
    };

    let identity_cache = load_identity_cache().await;
    let mut candidate_hashes: Vec<String> = Vec::new();
    for (hash, entry) in &identity_cache {
        let stored = match platform {
            RemoteModPlatform::Modrinth => entry.modrinth.as_deref(),
            RemoteModPlatform::CurseForge => entry.curseforge.as_deref(),
        };
        if stored == Some(project_id) {
            candidate_hashes.push(hash.clone());
        }
    }
    if candidate_hashes.is_empty() {
        return empty;
    }

    let mods_folder = storage::LOCATIONS
        .instances
        .get_instance_root(instance_id)
        .join("mods");
    let files: Vec<PathBuf> = mods_folder
        .read_dir()
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect()
        })
        .unwrap_or_default();

    let mut matched: Vec<PathBuf> = Vec::new();
    for path in files {
        let Ok(hash) = sha512_file(&path) else {
            continue;
        };
        if candidate_hashes.iter().any(|candidate| candidate == &hash) {
            matched.push(path);
        }
    }
    if matched.is_empty() {
        return empty;
    }

    let mut modrinth_cache = load_cache::<RemoteCacheEntry>(MODRINTH_CACHE).await;
    let mut curseforge_cache = load_cache::<RemoteCacheEntry>(CURSEFORGE_CACHE).await;
    let mut modrinth_identity: HashMap<String, Option<String>> = HashMap::new();
    let mut curseforge_identity: HashMap<String, Option<String>> = HashMap::new();
    let mut files_by_hash: HashMap<String, PathBuf> = HashMap::new();
    for path in &matched {
        if let Ok(hash) = sha512_file(path) {
            files_by_hash.insert(hash, path.clone());
        }
    }

    let mut result_mods = Vec::new();
    let mut modrinth_dirty = false;
    let mut curseforge_dirty = false;
    for (hash, path) in &files_by_hash {
        let mut parsed = match parse_mod(path) {
            Ok(mods) => mods,
            Err(crate::error::Error::NotAModFile) => continue,
            Err(error) => {
                warn!("Failed to parse mod {:?}: {error}", path);
                continue;
            }
        };
        let (info, identity) = resolve_platform_single(
            hash,
            platform,
            &mut modrinth_cache,
            &mut curseforge_cache,
            &files_by_hash,
        )
        .await;
        match platform {
            RemoteModPlatform::Modrinth => {
                modrinth_identity.extend(identity);
                modrinth_dirty = true;
            }
            RemoteModPlatform::CurseForge => {
                curseforge_identity.extend(identity);
                curseforge_dirty = true;
            }
        }
        if let Some(info) = info {
            for mod_info in &mut parsed {
                merge_remote(mod_info, &info);
            }
        }
        result_mods.extend(parsed);
    }

    if modrinth_dirty {
        merge_and_save(MODRINTH_CACHE, &modrinth_cache).await;
    }
    if curseforge_dirty {
        merge_and_save(CURSEFORGE_CACHE, &curseforge_cache).await;
    }
    merge_and_save_identity(modrinth_identity, curseforge_identity).await;

    ModInstalledInfo {
        installed: true,
        mods: result_mods,
    }
}

/// Merge online info into a locally-parsed mod. Online fields win; local data
/// fills whatever the platform does not provide.
fn merge_remote(mod_info: &mut ResolvedMod, remote: &RemoteModInfo) {
    if !remote.name.is_empty() {
        mod_info.name = remote.name.clone();
    }
    if let Some(description) = &remote.description
        && !description.is_empty()
    {
        mod_info.description = Some(description.clone());
    }
    if let Some(icon_url) = &remote.icon_url
        && !icon_url.is_empty()
    {
        mod_info.icon = Some(icon_url.clone());
    }
    if let Some(version_number) = &remote.version_number
        && !version_number.is_empty()
    {
        mod_info.version = Some(version_number.clone());
    }
    if !remote.authors.is_empty() {
        mod_info.authors = remote
            .authors
            .iter()
            .map(|name| ResolvedAuthorInfo {
                name: name.clone(),
                contact: None,
            })
            .collect();
    }
    if mod_info.loader == ModLoader::Unknown {
        let loaders = &remote.loaders;
        if loaders.iter().any(|loader| loader == "quilt") {
            mod_info.loader = ModLoader::Quilt;
        } else if loaders.iter().any(|loader| loader == "fabric") {
            mod_info.loader = ModLoader::Fabric;
        } else if loaders
            .iter()
            .any(|loader| loader == "forge" || loader == "neoforge")
        {
            mod_info.loader = ModLoader::Forge;
        }
    }
    mod_info.source = Some(remote.platform);
    mod_info.source_id = Some(remote.project_id.clone());
    mod_info.version_id = remote.version_id.clone();
}

pub async fn parse_mods(instance_id: &str) -> Vec<ResolvedMod> {
    let mods_folder = storage::LOCATIONS
        .instances
        .get_instance_root(instance_id)
        .join("mods");
    parse_folder_with_remote(&mods_folder).await
}

pub async fn check_installed(
    instance_id: &str,
    platform: RemoteModPlatform,
    project_id: &str,
) -> ModInstalledInfo {
    check_mod_installed(instance_id, platform, project_id).await
}

/// Delete the given files from an instance.
///
/// Only files under the instance root are accepted; paths outside it are
/// skipped with a warning so the launcher never deletes arbitrary user data. The
/// frontend only shows the remove action for mods, but the check is done
/// against the whole instance root so partial-download cleanup stays possible.
pub fn remove_mod_files(instance_id: &str, files: Vec<String>) -> Result<()> {
    let instance_root = storage::LOCATIONS.instances.get_instance_root(instance_id);
    let instance_root_canonical = instance_root.canonicalize().unwrap_or(instance_root);
    for file in files {
        let path = PathBuf::from(file);
        let canonical = path.canonicalize().unwrap_or(path);
        if !canonical.starts_with(&instance_root_canonical) {
            warn!(
                "Refusing to remove file outside instance root: {:?}",
                canonical
            );
            continue;
        }
        if let Err(error) = std::fs::remove_file(&canonical) {
            warn!("Failed to remove file {:?}: {error}", canonical);
        }
    }
    Ok(())
}
