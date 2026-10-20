// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Mods: local parsing, the offline parse cache and the merge of platform data
//! into a mod entry.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use log::warn;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{RemoteContentInfo, resolve_hashes};
use crate::index;
use crate::kind::{ContentKind, ContentScope};
use crate::mods::{
    ModLoader, ResolvedAuthorInfo, ResolvedMod, decode_icon_data_url, encode_icon,
    is_disabled_file, parse_mod,
};

const LOCAL_CACHE: &str = "local.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalCacheEntry {
    mods: Vec<ResolvedMod>,
}

type LocalCache = HashMap<String, LocalCacheEntry>;

fn local_cache_dir() -> PathBuf {
    storage::LOCATIONS.launcher.cache.join("mods")
}

fn icon_path(hash: &str) -> PathBuf {
    local_cache_dir()
        .join("local-icons")
        .join(format!("{hash}.png"))
}

async fn load_local_cache<T: DeserializeOwned>(name: &str) -> HashMap<String, T> {
    let path = local_cache_dir().join(name);
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

async fn save_local_cache<T: Serialize>(name: &str, cache: &HashMap<String, T>) {
    let Ok(bytes) = serde_json::to_vec(cache) else {
        warn!("Failed to serialize mod cache {name}");
        return;
    };
    if let Err(error) = tokio::fs::write(local_cache_dir().join(name), bytes).await {
        warn!("Failed to save mod cache {name}: {error}");
    }
}

async fn merge_local_cache<T>(name: &str, updates: &HashMap<String, T>)
where
    T: Clone + Serialize + DeserializeOwned,
{
    if updates.is_empty() {
        return;
    }
    let _guard = super::cache_write_lock().await;
    let mut cache = load_local_cache(name).await;
    for (hash, entry) in updates {
        cache.insert(hash.clone(), entry.clone());
    }
    save_local_cache(name, &cache).await;
}

pub async fn parse_mods(instance_id: &str) -> Vec<ResolvedMod> {
    let scope = ContentScope::new(instance_id, ContentKind::Mods);
    let folder = scope.folder();
    if let Err(error) = tokio::fs::create_dir_all(local_cache_dir()).await {
        warn!("Failed to create mod cache directory: {error}");
        return Vec::new();
    }

    let previous = index::load(&scope).await;
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
            return Vec::new();
        }
    };
    if scan.index != previous_snapshot {
        index::save(&scope, &scan.index).await;
    }

    let local_cache = load_local_cache::<LocalCacheEntry>(LOCAL_CACHE).await;
    let mut files_with_mods: Vec<(String, PathBuf, Vec<ResolvedMod>)> = Vec::new();
    let mut files_by_hash: HashMap<String, PathBuf> = HashMap::new();
    let mut new_local_entries: LocalCache = HashMap::new();

    for index::ScannedFile { path, sha512: hash } in scan.indexed {
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
        if !local_cache.contains_key(&hash) || own_icon(&parsed).is_some() {
            new_local_entries.insert(
                hash.clone(),
                LocalCacheEntry {
                    mods: parsed.clone(),
                },
            );
        }
        let disabled = is_disabled_file(&path);
        for mod_info in &mut parsed {
            mod_info.disabled = disabled;
            mod_info.path = path.clone();
        }
        files_by_hash.insert(hash.clone(), path.clone());
        files_with_mods.push((hash, path, parsed));
    }

    write_icons(&new_local_entries).await;
    merge_local_cache(LOCAL_CACHE, &new_local_entries).await;

    let hashes: Vec<String> = files_with_mods
        .iter()
        .map(|(hash, ..)| hash.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let remote_by_hash = resolve_hashes(&hashes, &files_by_hash).await;

    let mut result = Vec::new();
    for (hash, _, mut mods) in files_with_mods {
        let remote = remote_by_hash.get(&hash);
        let local_icon = if remote
            .is_none_or(|remote| remote.icon_url.as_deref().unwrap_or_default().is_empty())
        {
            read_icon_file(&hash).await
        } else {
            None
        };
        for mod_info in &mut mods {
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

async fn write_icons(entries: &LocalCache) {
    if entries.is_empty() {
        return;
    }
    if let Err(error) = tokio::fs::create_dir_all(local_cache_dir().join("local-icons")).await {
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

fn own_icon(mods: &[ResolvedMod]) -> Option<&str> {
    mods.iter()
        .find(|mod_info| !mod_info.embedded)
        .and_then(|mod_info| mod_info.icon.as_deref())
}

async fn read_icon_file(hash: &str) -> Option<String> {
    let bytes = tokio::fs::read(icon_path(hash)).await.ok()?;
    Some(encode_icon(bytes))
}

/// Merge online info into a locally-parsed mod. Online fields win; local data
/// fills whatever the platform does not provide.
pub(crate) fn merge_remote(mod_info: &mut ResolvedMod, remote: &RemoteContentInfo) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::RemotePlatform;
    use std::path::Path;

    #[test]
    fn a_remote_name_replaces_the_local_one() {
        let mut info = ResolvedMod::unrecognized(Path::new("example.jar"));
        info.name = "local".into();
        merge_remote(
            &mut info,
            &RemoteContentInfo {
                platform: RemotePlatform::Modrinth,
                project_id: "project".into(),
                version_id: Some("version".into()),
                name: "remote".into(),
                description: None,
                icon_url: None,
                authors: Vec::new(),
                download_url: None,
                version_number: Some("1.0".into()),
                loaders: vec!["fabric".into()],
                game_versions: Vec::new(),
            },
        );
        assert_eq!(info.name, "remote");
        assert_eq!(info.version.as_deref(), Some("1.0"));
        assert_eq!(info.loader, ModLoader::Fabric);
        assert_eq!(info.version_id.as_deref(), Some("version"));
    }
}
