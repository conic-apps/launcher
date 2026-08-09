// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Online mod metadata lookup with a persistent on-disk cache.
//!
//! Three cache files live under [`folder::DATA_LOCATION`]'s cache directory:
//!
//! - `modrinth.json` and `curseforge.json` cache the result of each platform's
//!   file-feature lookup, keyed by the file's SHA-512 checksum. Entries expire
//!   after [`REMOTE_CACHE_TTL_SECS`] and only carry list-display fields.
//! - `local.json` caches the local parse result so parsing is skipped on the
//!   next run. It never expires.
//!
//! Lookup order is Modrinth first (faster), then CurseForge. Online info takes
//! priority over the local parse result; local data supplements whatever the
//! platform does not provide. Icons found online are passed through as URLs.

use std::{
    collections::HashMap,
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha512};
use tauri::command;

use crate::mods::{ModLoader, ResolvedAuthorInfo, ResolvedMod, is_disabled_file, parse_mod};

/// The platform an online mod lookup was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteModPlatform {
    Modrinth,
    CurseForge,
}

/// The online metadata of a mod, as reported by its source platform.
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

/// SHA-512 checksum of a file, hex-encoded.
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

/// How long a Modrinth/CurseForge lookup result stays valid.
const REMOTE_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

const MODRINTH_CACHE: &str = "modrinth.json";
const CURSEFORGE_CACHE: &str = "curseforge.json";
const LOCAL_CACHE: &str = "local.json";

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

type RemoteCache = HashMap<String, RemoteCacheEntry>;
type LocalCache = HashMap<String, LocalCacheEntry>;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn cache_dir() -> std::path::PathBuf {
    folder::DATA_LOCATION.cache.join("mods")
}

async fn load_remote_cache(name: &str) -> RemoteCache {
    let path = cache_dir().join(name);
    match async_fs::read(&path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => RemoteCache::default(),
    }
}

async fn save_remote_cache(name: &str, cache: &RemoteCache) {
    let Ok(bytes) = serde_json::to_vec(cache) else {
        warn!("Failed to serialize mod cache {name}");
        return;
    };
    if let Err(error) = async_fs::write(cache_dir().join(name), bytes).await {
        warn!("Failed to save mod cache {name}: {error}");
    }
}

async fn load_local_cache() -> LocalCache {
    let path = cache_dir().join(LOCAL_CACHE);
    match async_fs::read(&path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => LocalCache::default(),
    }
}

async fn save_local_cache(cache: &LocalCache) {
    let Ok(bytes) = serde_json::to_vec(cache) else {
        warn!("Failed to serialize local mod cache");
        return;
    };
    if let Err(error) = async_fs::write(cache_dir().join(LOCAL_CACHE), bytes).await {
        warn!("Failed to save local mod cache: {error}");
    }
}

/// Resolve every mod inside a folder, merging online info when available.
///
/// This is the entry point the frontend calls to list all mods of an instance.
pub async fn parse_folder_with_remote<S: AsRef<Path> + ?Sized>(folder: &S) -> Vec<ResolvedMod> {
    let folder = folder.as_ref();
    let files: Vec<std::path::PathBuf> = folder
        .read_dir()
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect()
        })
        .unwrap_or_default();

    if let Err(error) = async_fs::create_dir_all(cache_dir()).await {
        warn!("Failed to create mod cache directory: {error}");
        return Vec::new();
    }

    let mut modrinth_cache = load_remote_cache(MODRINTH_CACHE).await;
    let mut curseforge_cache = load_remote_cache(CURSEFORGE_CACHE).await;
    let mut local_cache = load_local_cache().await;

    let timestamp = now();
    // One entry per file, so files that share the same content (and therefore
    // the same cache key) still show up individually with their own `disabled`
    // flag.
    let mut files_with_mods: Vec<(String, std::path::PathBuf, Vec<ResolvedMod>)> = Vec::new();
    let mut files_by_hash: HashMap<String, std::path::PathBuf> = HashMap::new();
    let mut local_cache_dirty = false;

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
                    vec![ResolvedMod::unrecognized(path.file_name())]
                }
            },
        };
        if parsed.is_empty() {
            continue;
        }
        if !local_cache.contains_key(&hash) {
            local_cache.insert(
                hash.clone(),
                LocalCacheEntry {
                    mods: parsed.clone(),
                },
            );
            local_cache_dirty = true;
        }
        // `disabled` is a property of the file name, not the content, so it
        // must be derived from the current file even when the parse result
        // came from the cache.
        let disabled = is_disabled_file(&path);
        for mod_info in &mut parsed {
            mod_info.disabled = disabled;
        }
        files_by_hash.insert(hash.clone(), path.clone());
        files_with_mods.push((hash, path, parsed));
    }

    if local_cache_dirty {
        save_local_cache(&local_cache).await;
    }

    // Batch the platform lookups for every file whose cache entry is stale.
    let mut remote_by_hash: HashMap<String, RemoteModInfo> = HashMap::new();
    let mut needs_modrinth: Vec<String> = Vec::new();
    for (hash, ..) in &files_with_mods {
        if let Some(entry) = modrinth_cache.get(hash)
            && timestamp.saturating_sub(entry.timestamp) < REMOTE_CACHE_TTL_SECS
        {
            remote_by_hash.insert(hash.clone(), entry.info.clone());
        } else if let Some(entry) = curseforge_cache.get(hash)
            && timestamp.saturating_sub(entry.timestamp) < REMOTE_CACHE_TTL_SECS
        {
            remote_by_hash.insert(hash.clone(), entry.info.clone());
        } else {
            needs_modrinth.push(hash.clone());
        }
    }

    if !needs_modrinth.is_empty() {
        query_modrinth_batch(&needs_modrinth, &mut modrinth_cache, &mut remote_by_hash).await;
    }

    let needs_curseforge: Vec<String> = files_with_mods
        .iter()
        .map(|(hash, ..)| hash)
        .filter(|hash| !remote_by_hash.contains_key(*hash))
        .cloned()
        .collect();
    if !needs_curseforge.is_empty() {
        query_curseforge_batch(
            &needs_curseforge,
            &files_by_hash,
            &mut curseforge_cache,
            &mut remote_by_hash,
        )
        .await;
    }

    save_remote_cache(MODRINTH_CACHE, &modrinth_cache).await;
    save_remote_cache(CURSEFORGE_CACHE, &curseforge_cache).await;

    // Merge online info into the local parse results. Multiple files may share
    // the same hash (identical content), so the entry is kept for the rest.
    let mut result = Vec::new();
    for (hash, _, mut mods) in files_with_mods {
        if let Some(remote) = remote_by_hash.get(&hash) {
            for mod_info in &mut mods {
                merge_remote(mod_info, remote);
            }
        }
        result.extend(mods);
    }
    result
}

/// Query Modrinth for a batch of SHA-512 hashes and store the fresh results
/// in the cache. Runs until the first request error, which only logs a warning.
async fn query_modrinth_batch(
    hashes: &[String],
    cache: &mut RemoteCache,
    remote_by_hash: &mut HashMap<String, RemoteModInfo>,
) {
    let versions = match modrinth::get_versions_from_hashes(hashes, "sha512").await {
        Ok(versions) => versions,
        Err(error) => {
            warn!("Failed to look up mods on Modrinth: {error}");
            return;
        }
    };
    if versions.is_empty() {
        return;
    }

    // Fetch the matching projects in one request.
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
            return;
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
        let members = modrinth::get_project_members(team).await;
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
        remote_by_hash.insert(hash.clone(), info);
    }
}

/// Query CurseForge for a batch of fingerprints and store the fresh results
/// in the cache. Runs until the first request error, which only logs a warning.
async fn query_curseforge_batch(
    hashes: &[String],
    files_by_hash: &HashMap<String, std::path::PathBuf>,
    cache: &mut RemoteCache,
    remote_by_hash: &mut HashMap<String, RemoteModInfo>,
) {
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
        return;
    }

    let fingerprints: Vec<u32> = fingerprint_of.keys().copied().collect();
    let value =
        match curseforge::get_fingerprint_matches(curseforge::MINECRAFT_GAME_ID, &fingerprints)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                warn!("Failed to look up mods on CurseForge: {error}");
                return;
            }
        };
    let Some(exact_matches) = value
        .pointer("/data/exactMatches")
        .and_then(serde_json::Value::as_array)
    else {
        return;
    };

    let timestamp = now();
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
        remote_by_hash.insert(hash, info);
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

/// Tauri command: list every mod of an instance, merged with online info.
#[command]
pub(crate) async fn cmd_parse_mods(instance_id: String) -> Vec<ResolvedMod> {
    let mods_folder = folder::DATA_LOCATION
        .get_instance_root(&instance_id)
        .join("mods");
    parse_folder_with_remote(&mods_folder).await
}
