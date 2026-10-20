// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Resource packs: the local listing, its parse cache and the platform identity
//! shared with mods. Local names and icons stay local; the remote data is only
//! used to identify the archive and its version.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use log::warn;
use serde::{Serialize, de::DeserializeOwned};

use super::resolve_hashes;
use crate::index;
use crate::kind::{ContentKind, ContentScope};
use crate::mods::{decode_icon_data_url, encode_icon};
use crate::resourcepack::{Resourcepack, parse_resourcepack};

const LOCAL_CACHE: &str = "local.json";

type LocalCache = HashMap<String, Resourcepack>;

fn local_cache_dir() -> PathBuf {
    storage::LOCATIONS.launcher.cache.join("resourcepacks")
}

fn icon_path(hash: &str) -> PathBuf {
    local_cache_dir().join("icons").join(format!("{hash}.png"))
}

async fn load_local_cache<T: DeserializeOwned>(name: &str) -> HashMap<String, T> {
    let path = local_cache_dir().join(name);
    match tokio::fs::read(&path).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(cache) => cache,
            Err(error) => {
                warn!(
                    "{} could not be read as a resource pack cache ({error}); it will be rebuilt",
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
        warn!("Failed to serialize resource pack cache {name}");
        return;
    };
    if let Err(error) = tokio::fs::write(local_cache_dir().join(name), bytes).await {
        warn!("Failed to save resource pack cache {name}: {error}");
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

pub async fn parse_resourcepacks(instance_id: &str) -> Vec<Resourcepack> {
    let scope = ContentScope::new(instance_id, ContentKind::ResourcePacks);
    let folder = scope.folder();
    if let Err(error) = tokio::fs::create_dir_all(local_cache_dir()).await {
        warn!("Failed to create resource pack cache directory: {error}");
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

    let local_cache = load_local_cache::<Resourcepack>(LOCAL_CACHE).await;
    let mut packs = Vec::new();
    let mut files_by_hash: HashMap<String, PathBuf> = HashMap::new();
    let mut new_local_entries: LocalCache = HashMap::new();

    for index::ScannedFile { path, sha512: hash } in scan.indexed {
        let mut pack = match local_cache.get(&hash).cloned() {
            Some(mut pack) => {
                pack.path = path.clone();
                pack.icon = read_icon_file(&hash).await;
                pack
            }
            None => match parse_resourcepack(path.as_os_str()) {
                Ok(pack) => {
                    write_icon(&hash, pack.icon.as_deref()).await;
                    new_local_entries.insert(
                        hash.clone(),
                        Resourcepack {
                            icon: None,
                            ..pack.clone()
                        },
                    );
                    pack
                }
                Err(error) => {
                    warn!("Failed to parse resource pack {:?}: {error}", path);
                    continue;
                }
            },
        };
        pack.path = path.clone();
        files_by_hash.insert(hash, path);
        packs.push(pack);
    }

    for path in scan.directories.into_iter().chain(scan.other_files) {
        match parse_resourcepack(path.as_os_str()) {
            Ok(pack) => packs.push(pack),
            Err(error) => log::debug!("{} is not a usable resource pack: {error}", path.display()),
        }
    }

    merge_local_cache(LOCAL_CACHE, &new_local_entries).await;

    let hashes: Vec<String> = files_by_hash
        .keys()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    resolve_hashes(&hashes, &files_by_hash).await;

    packs.sort_by(|left, right| left.path.cmp(&right.path));
    packs
}

async fn write_icon(hash: &str, icon: Option<&str>) {
    let Some(bytes) = icon.and_then(decode_icon_data_url) else {
        return;
    };
    if let Err(error) = tokio::fs::create_dir_all(local_cache_dir().join("icons")).await {
        warn!("Failed to create the resource pack icon cache directory: {error}");
        return;
    }
    if let Err(error) = tokio::fs::write(icon_path(hash), bytes).await {
        warn!("Failed to save the cached icon of {hash}: {error}");
    }
}

async fn read_icon_file(hash: &str) -> Option<String> {
    let bytes = tokio::fs::read(icon_path(hash)).await.ok()?;
    Some(encode_icon(bytes))
}
