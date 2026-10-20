// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Platform metadata lookup shared by every content kind.
//!
//! Three JSON files live under `cache/remote/`:
//!
//! - `modrinth.json` and `curseforge.json` cache each platform's file-feature
//!   lookup, keyed by the file's SHA-512 checksum. Entries expire after the
//!   cache TTL and only carry list-display fields.
//! - `identity.json` maps each checksum to the project/mod and version/file ids
//!   it is known on either platform. It never expires and does not carry the
//!   lookup details. A failed lookup leaves the entry unchanged.
//!
//! The per-kind child modules own their local parse cache, their icon sidecar
//! and their path index:
//!
//! - `mods` keeps `cache/mods/local.json` and `cache/mods/local-icons/`;
//! - `resourcepacks` keeps `cache/resourcepacks/local.json` and
//!   `cache/resourcepacks/icons/`.
//!
//! The path index is shared through `crate::index` and stored under each
//! kind's cache folder, so an unchanged archive is not hashed again.
//!
//! The platform lookups for a file are classified by its identity entry:
//!
//! - When the identity carries both ids, the lookup follows Modrinth first
//!   (faster), then CurseForge — cache first, then a fresh query.
//! - Otherwise both platforms are queried at the same time. The first response
//!   wins for the frontend result, but both responses are fully processed and
//!   written to the caches. A platform that fails or has no data for the file
//!   leaves its identity untouched; the next parse retries it.
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
//! `merge_and_save` and the per-kind local caches take the same lock, and that
//! is what keeps the writes deadlock-free. Three invariants hold it together:
//!
//! 1. One acquisition, never nested. A `tokio::sync::Mutex` is not reentrant, so
//!    the helpers that run under it only load and save files, which take no lock
//!    of their own — no cycle is reachable.
//! 2. Nothing but disk I/O and serde runs under the lock. No platform query
//!    reaches back into it; every query happens before the saves.
//! 3. The synchronous scan that hashes a folder runs outside the lock, before
//!    anything is saved.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use futures::{FutureExt, select};
use log::warn;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::error::Result;
use crate::index;
pub use crate::kind::{ContentKind, ContentScope, RemotePlatform};

pub mod mods;
pub mod resourcepacks;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteContentInfo {
    pub platform: RemotePlatform,
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

const REMOTE_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

const MODRINTH_CACHE: &str = "modrinth.json";
const CURSEFORGE_CACHE: &str = "curseforge.json";
const IDENTITY_CACHE: &str = "identity.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteCacheEntry {
    #[serde(rename = "ts")]
    timestamp: u64,
    #[serde(flatten)]
    info: RemoteContentInfo,
}

/// The ids one platform knows a file by. The version/file id is the identity of
/// the exact bytes, not just of the project, which is what lets a later version
/// switch tell two installed versions apart.
#[derive(Debug, Clone)]
struct PlatformIdentity {
    project_id: String,
    version_id: Option<String>,
}

/// The platform ids a file is known on. A missing field means either that the
/// platform has no match or that the lookup has not succeeded yet; a failure
/// leaves the entry unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct IdentityEntry {
    #[serde(default)]
    modrinth: Option<String>,
    #[serde(default)]
    modrinth_version: Option<String>,
    #[serde(default)]
    curseforge: Option<String>,
    #[serde(default)]
    curseforge_file: Option<String>,
}

impl IdentityEntry {
    fn remember(&mut self, platform: RemotePlatform, update: &PlatformIdentity) {
        match platform {
            RemotePlatform::Modrinth => {
                self.modrinth = Some(update.project_id.clone());
                if update.version_id.is_some() {
                    self.modrinth_version = update.version_id.clone();
                }
            }
            RemotePlatform::CurseForge => {
                self.curseforge = Some(update.project_id.clone());
                if update.version_id.is_some() {
                    self.curseforge_file = update.version_id.clone();
                }
            }
        }
    }

    fn project_and_version(&self, platform: RemotePlatform) -> Option<(&str, Option<&str>)> {
        match platform {
            RemotePlatform::Modrinth => self
                .modrinth
                .as_deref()
                .map(|project| (project, self.modrinth_version.as_deref())),
            RemotePlatform::CurseForge => self
                .curseforge
                .as_deref()
                .map(|project| (project, self.curseforge_file.as_deref())),
        }
    }
}

type RemoteCache = HashMap<String, RemoteCacheEntry>;
type IdentityCache = HashMap<String, IdentityEntry>;
type IdentityUpdates = HashMap<String, Option<PlatformIdentity>>;

/// Serializes the read-merge-write of the shared cache files so concurrent
/// parse requests (e.g. the frontend switching instances mid-parse) never
/// overwrite each other's entries. See the module docs for the invariants that
/// keep taking it deadlock-free.
static CACHE_WRITE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

pub(crate) async fn cache_write_lock() -> tokio::sync::MutexGuard<'static, ()> {
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
    storage::LOCATIONS.launcher.cache.join("remote")
}

async fn load_cache<T: DeserializeOwned>(name: &str) -> HashMap<String, T> {
    let path = cache_dir().join(name);
    match tokio::fs::read(&path).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(cache) => cache,
            Err(error) => {
                warn!(
                    "{} could not be read as a content cache ({error}); it will be rebuilt",
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
        warn!("Failed to serialize content cache {name}");
        return;
    };
    if let Err(error) = tokio::fs::create_dir_all(cache_dir()).await {
        warn!("Failed to create the remote cache directory: {error}");
        return;
    }
    if let Err(error) = tokio::fs::write(cache_dir().join(name), bytes).await {
        warn!("Failed to save content cache {name}: {error}");
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

/// Merge the per-platform lookup outcomes into the identity cache and write it
/// back. Only successes are applied; a failed or empty lookup leaves the entry
/// unchanged.
///
/// The merge is field-wise rather than entry-wise because two concurrent parses
/// can discover the same file on different platforms, which is why it does not
/// go through `merge_and_save` like the other caches: there the entry *is* the
/// value, and here only some of its fields are.
async fn merge_and_save_identity(modrinth: IdentityUpdates, curseforge: IdentityUpdates) {
    let has_update =
        modrinth.values().any(Option::is_some) || curseforge.values().any(Option::is_some);
    if !has_update {
        return;
    }
    let _guard = cache_write_lock().await;
    let mut cache = load_identity_cache().await;
    let mut changed = false;
    for (hash, update) in modrinth {
        if let Some(update) = update {
            let entry = cache.entry(hash).or_default();
            let before = (entry.modrinth.clone(), entry.modrinth_version.clone());
            entry.remember(RemotePlatform::Modrinth, &update);
            changed |= entry.modrinth != before.0 || entry.modrinth_version != before.1;
        }
    }
    for (hash, update) in curseforge {
        if let Some(update) = update {
            let entry = cache.entry(hash).or_default();
            let before = (entry.curseforge.clone(), entry.curseforge_file.clone());
            entry.remember(RemotePlatform::CurseForge, &update);
            changed |= entry.curseforge != before.0 || entry.curseforge_file != before.1;
        }
    }
    if changed {
        save_identity_cache(&cache).await;
    }
}

fn cache_is_fresh(entry: &RemoteCacheEntry, timestamp: u64) -> bool {
    timestamp.saturating_sub(entry.timestamp) < REMOTE_CACHE_TTL_SECS
}

/// Query Modrinth for a batch of SHA-512 hashes and store the fresh results
/// in the cache. Runs until the first request error, which only logs a warning.
///
/// Returns the resolved content keyed by hash and, for every requested hash,
/// the Modrinth identity (`Some`) or `None` when the lookup failed or found no
/// data.
async fn query_modrinth_batch(
    hashes: &[String],
    cache: &mut RemoteCache,
) -> (HashMap<String, RemoteContentInfo>, IdentityUpdates) {
    let mut identity: IdentityUpdates = hashes.iter().map(|hash| (hash.clone(), None)).collect();
    if hashes.is_empty() {
        return (HashMap::new(), identity);
    }
    let versions = match modrinth::get_versions_from_hashes(hashes, "sha512").await {
        Ok(versions) => versions,
        Err(error) => {
            warn!("Failed to look up content on Modrinth: {error}");
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
    let mut info_by_hash: HashMap<String, RemoteContentInfo> = HashMap::new();
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
        let info = RemoteContentInfo {
            platform: RemotePlatform::Modrinth,
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
        identity.insert(hash.clone(), Some(platform_identity(&info)));
    }
    (info_by_hash, identity)
}

fn platform_identity(info: &RemoteContentInfo) -> PlatformIdentity {
    PlatformIdentity {
        project_id: info.project_id.clone(),
        version_id: info.version_id.clone(),
    }
}

/// Resolves the platform metadata for a batch of checksums, updating the shared
/// remote and identity caches. The caller merges the returned info into its own
/// parsed representation; the lookup itself is the same for every content kind.
pub(crate) async fn resolve_hashes(
    hashes: &[String],
    files_by_hash: &HashMap<String, PathBuf>,
) -> HashMap<String, RemoteContentInfo> {
    if hashes.is_empty() {
        return HashMap::new();
    }

    let timestamp = now();
    let mut modrinth_cache = load_cache::<RemoteCacheEntry>(MODRINTH_CACHE).await;
    let mut curseforge_cache = load_cache::<RemoteCacheEntry>(CURSEFORGE_CACHE).await;
    let identity_cache = load_identity_cache().await;

    let mut group_a: Vec<String> = Vec::new();
    let mut group_b: Vec<String> = Vec::new();
    for hash in hashes {
        match identity_cache.get(hash) {
            Some(identity) if identity.modrinth.is_some() && identity.curseforge.is_some() => {
                group_a.push(hash.clone());
            }
            _ => group_b.push(hash.clone()),
        }
    }

    let mut remote_by_hash: HashMap<String, RemoteContentInfo> = HashMap::new();
    let mut modrinth_identity: IdentityUpdates = HashMap::new();
    let mut curseforge_identity: IdentityUpdates = HashMap::new();
    let mut modrinth_dirty = false;
    let mut curseforge_dirty = false;

    let mut needs_modrinth: Vec<String> = Vec::new();
    for hash in &group_a {
        let mut found = false;
        if let Some(entry) = modrinth_cache.get(hash) {
            modrinth_identity.insert(hash.clone(), Some(platform_identity(&entry.info)));
            if cache_is_fresh(entry, timestamp) {
                remote_by_hash.insert(hash.clone(), entry.info.clone());
                found = true;
            }
        }
        if let Some(entry) = curseforge_cache.get(hash) {
            curseforge_identity.insert(hash.clone(), Some(platform_identity(&entry.info)));
            if cache_is_fresh(entry, timestamp) && !found {
                remote_by_hash.insert(hash.clone(), entry.info.clone());
                found = true;
            }
        }
        if !found {
            needs_modrinth.push(hash.clone());
        }
    }

    if !needs_modrinth.is_empty() {
        let (info, identity) = query_modrinth_batch(&needs_modrinth, &mut modrinth_cache).await;
        for (hash, remote) in info {
            remote_by_hash.entry(hash).or_insert(remote);
        }
        modrinth_identity.extend(identity);
        modrinth_dirty = true;
    }

    let needs_curseforge: Vec<String> = group_a
        .iter()
        .filter(|hash| !remote_by_hash.contains_key(*hash))
        .cloned()
        .collect();
    if !needs_curseforge.is_empty() {
        let (info, identity) =
            query_curseforge_batch(&needs_curseforge, files_by_hash, &mut curseforge_cache).await;
        for (hash, remote) in info {
            remote_by_hash.entry(hash).or_insert(remote);
        }
        curseforge_identity.extend(identity);
        curseforge_dirty = true;
    }

    if !group_b.is_empty() {
        let mut needs_modrinth_b: Vec<String> = Vec::new();
        let mut needs_curseforge_b: Vec<String> = Vec::new();
        for hash in &group_b {
            match modrinth_cache.get(hash) {
                Some(entry) => {
                    modrinth_identity.insert(hash.clone(), Some(platform_identity(&entry.info)));
                    if cache_is_fresh(entry, timestamp) {
                        remote_by_hash
                            .entry(hash.clone())
                            .or_insert(entry.info.clone());
                    } else {
                        needs_modrinth_b.push(hash.clone());
                    }
                }
                None => needs_modrinth_b.push(hash.clone()),
            }
            match curseforge_cache.get(hash) {
                Some(entry) => {
                    curseforge_identity.insert(hash.clone(), Some(platform_identity(&entry.info)));
                    if cache_is_fresh(entry, timestamp) {
                        remote_by_hash
                            .entry(hash.clone())
                            .or_insert(entry.info.clone());
                    } else {
                        needs_curseforge_b.push(hash.clone());
                    }
                }
                None => needs_curseforge_b.push(hash.clone()),
            }
        }

        let mut modrinth_fut =
            Box::pin(query_modrinth_batch(&needs_modrinth_b, &mut modrinth_cache)).fuse();
        let mut curseforge_fut = Box::pin(query_curseforge_batch(
            &needs_curseforge_b,
            files_by_hash,
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

    remote_by_hash
}

/// Query CurseForge for a batch of fingerprints and store the fresh results
/// in the cache. Runs until the first request error, which only logs a warning.
///
/// Returns the resolved content keyed by hash and, for every requested hash,
/// the CurseForge identity (`Some`) or `None` when the lookup failed or found no
/// data.
async fn query_curseforge_batch(
    hashes: &[String],
    files_by_hash: &HashMap<String, PathBuf>,
    cache: &mut RemoteCache,
) -> (HashMap<String, RemoteContentInfo>, IdentityUpdates) {
    let mut identity: IdentityUpdates = hashes.iter().map(|hash| (hash.clone(), None)).collect();
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
                warn!("Failed to look up content on CurseForge: {error}");
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
    let mut info_by_hash: HashMap<String, RemoteContentInfo> = HashMap::new();
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
                warn!("Failed to fetch CurseForge project {mod_id}: {error}");
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
        let info = RemoteContentInfo {
            platform: RemotePlatform::CurseForge,
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
        identity.insert(hash.clone(), Some(platform_identity(&info)));
    }
    (info_by_hash, identity)
}

/// One installed archive the open project resolved to.
#[derive(Debug, Clone)]
pub struct InstalledFile {
    pub path: PathBuf,
    pub version: Option<String>,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InstalledContent {
    pub installed: bool,
    pub files: Vec<InstalledFile>,
}

/// Checks whether a project is installed in the scope's folder.
///
/// Only the indexed files are considered: for resource packs that means zips,
/// so a directory pack is deliberately invisible to the installed check. The
/// path index avoids re-hashing unchanged archives, and identity avoids any
/// platform request.
pub async fn check_installed(
    scope: &ContentScope,
    platform: RemotePlatform,
    project_id: &str,
) -> InstalledContent {
    let empty = InstalledContent {
        installed: false,
        files: Vec::new(),
    };

    let identity_cache = load_identity_cache().await;
    let has_candidate = identity_cache.values().any(|entry| {
        entry
            .project_and_version(platform)
            .is_some_and(|(project, _)| project == project_id)
    });
    if !has_candidate {
        return empty;
    }

    let folder = scope.folder();
    let previous = index::load(scope).await;
    let previous_snapshot = previous.clone();
    let scan_scope = scope.clone();
    let scan_folder = folder.clone();
    let scan = match tokio::task::spawn_blocking(move || {
        index::scan(&scan_scope, &scan_folder, &previous)
    })
    .await
    {
        Ok(scan) => scan,
        Err(error) => {
            warn!("Scanning {:?} did not run: {error}", folder);
            return empty;
        }
    };
    if scan.index != previous_snapshot {
        index::save(scope, &scan.index).await;
    }
    if scan.indexed.is_empty() {
        return empty;
    }

    let modrinth_cache = load_cache::<RemoteCacheEntry>(MODRINTH_CACHE).await;
    let curseforge_cache = load_cache::<RemoteCacheEntry>(CURSEFORGE_CACHE).await;

    let mut files = Vec::new();
    for scanned in scan.indexed {
        let Some(identity) = identity_cache.get(&scanned.sha512) else {
            continue;
        };
        let Some((stored_project, version_id)) = identity.project_and_version(platform) else {
            continue;
        };
        if stored_project != project_id {
            continue;
        }
        let version = match platform {
            RemotePlatform::Modrinth => modrinth_cache
                .get(&scanned.sha512)
                .and_then(|entry| entry.info.version_number.clone()),
            RemotePlatform::CurseForge => curseforge_cache
                .get(&scanned.sha512)
                .and_then(|entry| entry.info.version_number.clone()),
        };
        files.push(InstalledFile {
            path: scanned.path,
            version,
            version_id: version_id.map(str::to_string),
        });
    }

    InstalledContent {
        installed: !files.is_empty(),
        files,
    }
}

/// Records the identity of a file the launcher just installed.
///
/// The checksum is the one the index will use, so the check that follows the
/// install does not hash the archive a second time.
pub async fn record_installed(
    scope: &ContentScope,
    platform: RemotePlatform,
    project_id: &str,
    version_id: Option<&str>,
    path: &Path,
) {
    let path_buf = path.to_path_buf();
    let sha512 = match tokio::task::spawn_blocking(move || index::sha512_file(&path_buf)).await {
        Ok(Ok(sha512)) => sha512,
        Ok(Err(error)) => {
            warn!("Failed to hash the installed content: {error}");
            return;
        }
        Err(error) => {
            warn!("Hashing the installed content did not run: {error}");
            return;
        }
    };
    index::remember(scope, path, &sha512).await;

    let update = PlatformIdentity {
        project_id: project_id.to_string(),
        version_id: version_id.map(str::to_string),
    };
    let mut modrinth = IdentityUpdates::new();
    let mut curseforge = IdentityUpdates::new();
    match platform {
        RemotePlatform::Modrinth => {
            modrinth.insert(sha512, Some(update));
        }
        RemotePlatform::CurseForge => {
            curseforge.insert(sha512, Some(update));
        }
    }
    merge_and_save_identity(modrinth, curseforge).await;
}

/// Delete the given files from an instance.
///
/// Only files under the instance root are accepted; paths outside it are
/// skipped with a warning so the launcher never deletes arbitrary user data.
pub fn remove_content_files(instance_id: &str, files: Vec<String>) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_keeps_project_and_version_per_platform() {
        let mut identity = IdentityEntry::default();
        identity.remember(
            RemotePlatform::Modrinth,
            &PlatformIdentity {
                project_id: "modrinth-project".into(),
                version_id: Some("modrinth-version".into()),
            },
        );
        identity.remember(
            RemotePlatform::CurseForge,
            &PlatformIdentity {
                project_id: "123".into(),
                version_id: Some("456".into()),
            },
        );
        assert_eq!(
            identity.project_and_version(RemotePlatform::Modrinth),
            Some(("modrinth-project", Some("modrinth-version")))
        );
        assert_eq!(
            identity.project_and_version(RemotePlatform::CurseForge),
            Some(("123", Some("456")))
        );
    }

    #[test]
    fn a_versionless_success_keeps_the_known_version() {
        let mut identity = IdentityEntry::default();
        identity.remember(
            RemotePlatform::Modrinth,
            &PlatformIdentity {
                project_id: "project".into(),
                version_id: Some("version".into()),
            },
        );
        identity.remember(
            RemotePlatform::Modrinth,
            &PlatformIdentity {
                project_id: "project".into(),
                version_id: None,
            },
        );
        assert_eq!(
            identity.project_and_version(RemotePlatform::Modrinth),
            Some(("project", Some("version")))
        );
    }
}
