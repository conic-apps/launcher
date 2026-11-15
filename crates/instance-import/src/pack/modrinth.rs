// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Modrinth pack format (`.mrpack`): a `modrinth.index.json` that names
//! every file with its own download URL and hash, plus the `overrides/` and
//! `client-overrides/` trees.
//!
//! Nothing is looked up per project: the index already carries the URLs, so the
//! API is only asked for a file whose `downloads` came up empty.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use instance::ModLoaderType;

use super::relative_path;
use crate::error::{Error, Result};
use crate::files::FileTree;
use crate::loader::strip_minecraft_prefix;
use crate::settings::Settings;
use crate::source::Source;
use crate::{Metadata, ResolvedFile, UnavailableFile};

const MANIFEST: &str = "modrinth.index.json";

/// The trees an index's files are written over, later winning over earlier.
const OVERRIDES: &[&str] = &["overrides", "client-overrides"];

/// The Modrinth half of an inspection: the file entries to resolve.
#[derive(Clone)]
pub(crate) struct Inspection {
    files: Vec<ManifestFile>,
    trees: Vec<FileTree>,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "versionId")]
    version_id: String,
    #[serde(default)]
    files: Vec<ManifestFile>,
    #[serde(default)]
    dependencies: HashMap<String, String>,
}

#[derive(Clone, Deserialize)]
struct ManifestFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    /// Which sides of the game need the file; absent means both.
    #[serde(default)]
    env: Option<HashMap<String, String>>,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default, rename = "fileSize")]
    file_size: Option<u64>,
}

/// Reads a Modrinth index into the common metadata and its own half.
pub(super) fn inspect(path: &Path, source: &Source) -> Result<(Metadata, Inspection)> {
    let bytes = source.read(MANIFEST)?;
    let manifest = parse_manifest(&bytes)?;
    let minecraft = manifest
        .dependencies
        .get("minecraft")
        .map(|version| version.trim())
        .filter(|version| !version.is_empty())
        .ok_or_else(|| {
            Error::Manifest("the Modrinth index names no Minecraft version".to_string())
        })?
        .to_string();
    let (loader_type, loader_version) = parse_loader(&manifest.dependencies)?;
    let name = manifest.name.trim();
    let name = if name.is_empty() {
        super::fallback_name(path)
    } else {
        name.to_string()
    };
    log::info!(
        "import '{name}': Modrinth {}, Minecraft {minecraft}, {} index files",
        manifest.version_id,
        manifest.files.len()
    );
    let mut trees = Vec::new();
    for prefix in OVERRIDES {
        if source.has_tree(prefix)? {
            trees.push(source.tree(prefix));
        }
    }
    Ok((
        Metadata {
            name,
            minecraft,
            loader_type,
            loader_version,
            settings: Settings::default(),
        },
        Inspection {
            files: manifest.files,
            trees,
        },
    ))
}

/// Resolves the index's file entries into the common plan.
///
/// The one network case is an entry without a URL: its SHA-1 is looked up on
/// Modrinth, which is how a pack that lost its `downloads` is still installable.
pub(super) async fn resolve(inspection: Inspection) -> Result<crate::Resolved> {
    let mut files = Vec::new();
    let mut unavailable = Vec::new();
    let mut paths = HashSet::new();
    let mut without_url = Vec::new();
    for (index, file) in inspection.files.iter().enumerate() {
        // "optional" is a file the pack runs without; installing it keeps the
        // client-side pack the author exported, and only "unsupported" is left
        // out.
        let client = file
            .env
            .as_ref()
            .and_then(|env| env.get("client"))
            .map(String::as_str);
        if client == Some("unsupported") {
            log::info!(
                "skipping {}: the pack does not use it on a client",
                file.path
            );
            continue;
        }
        let Some(path) = relative_path(&file.path) else {
            unavailable.push(file.unavailable("the path is not inside the instance"));
            continue;
        };
        if !paths.insert(path.to_string_lossy().to_ascii_lowercase()) {
            unavailable.push(file.unavailable("another index entry already uses this path"));
            continue;
        }
        match file.url() {
            Some(url) => files.push(file.resolved(path, url)),
            None => without_url.push((index, path)),
        }
    }

    if !without_url.is_empty() {
        let missing: Vec<&ManifestFile> = without_url
            .iter()
            .filter_map(|(index, _)| inspection.files.get(*index))
            .collect();
        let by_hash = lookup_by_hash(&missing).await;
        for (index, path) in without_url {
            let file = &inspection.files[index];
            match file.hash_of("sha1").and_then(|hash| by_hash.get(hash)) {
                Some(url) => files.push(file.resolved(path, url.clone())),
                None => {
                    unavailable.push(file.unavailable("Modrinth has no download for this file"))
                }
            }
        }
    }

    Ok(crate::Resolved {
        files,
        unavailable,
        trees: inspection.trees,
    })
}

/// Asks Modrinth for the files behind the entries that carry no URL.
async fn lookup_by_hash(files: &[&ManifestFile]) -> HashMap<String, String> {
    let hashes: Vec<String> = files
        .iter()
        .filter_map(|file| file.hash_of("sha1").map(str::to_owned))
        .collect();
    if hashes.is_empty() {
        return HashMap::new();
    }
    let Ok(versions) = modrinth::get_versions_from_hashes(&hashes, "sha1").await else {
        return HashMap::new();
    };
    versions
        .into_iter()
        .filter_map(|(hash, version)| {
            let url = version
                .get("files")
                .and_then(serde_json::Value::as_array)?
                .iter()
                .find_map(|file| file.get("url").and_then(serde_json::Value::as_str))?;
            Some((hash, url.to_string()))
        })
        .collect()
}

impl ManifestFile {
    fn url(&self) -> Option<&str> {
        self.downloads
            .iter()
            .map(String::as_str)
            .find(|url| !url.trim().is_empty())
    }

    fn hash_of(&self, algorithm: &str) -> Option<&str> {
        self.hashes
            .get(algorithm)
            .map(String::as_str)
            .filter(|hash| !hash.is_empty())
    }

    fn resolved(&self, path: PathBuf, url: impl Into<String>) -> ResolvedFile {
        let checksum = [("sha1", 1), ("sha512", 2), ("sha256", 3)]
            .into_iter()
            .find_map(|(algorithm, kind)| {
                let value = self.hash_of(algorithm)?;
                Some(match kind {
                    1 => download::Checksum::Sha1(value.to_string()),
                    2 => download::Checksum::Sha512(value.to_string()),
                    _ => download::Checksum::Sha256(value.to_string()),
                })
            })
            .unwrap_or(download::Checksum::None);
        ResolvedFile {
            path,
            url: url.into(),
            size_bytes: self.file_size,
            checksum,
        }
    }

    fn unavailable(&self, reason: &str) -> UnavailableFile {
        UnavailableFile {
            file_name: file_name(&self.path),
            reference: self.path.clone(),
            required: self
                .env
                .as_ref()
                .and_then(|env| env.get("client"))
                .map(String::as_str)
                != Some("optional"),
            reason: reason.to_string(),
        }
    }
}

/// The map's loader, from whichever dependency names one.
fn parse_loader(
    dependencies: &HashMap<String, String>,
) -> Result<(Option<ModLoaderType>, Option<String>)> {
    let minecraft = dependencies
        .get("minecraft")
        .map(String::as_str)
        .unwrap_or_default();
    let mut loader = None;
    let mut version = None;
    for (key, value) in dependencies {
        let kind = match key.to_ascii_lowercase().as_str() {
            "minecraft" => continue,
            "forge" => ModLoaderType::Forge,
            "neoforge" | "neo-forge" => ModLoaderType::Neoforge,
            "fabric-loader" => ModLoaderType::Fabric,
            "quilt-loader" => ModLoaderType::Quilt,
            other => {
                log::warn!("ignoring the unknown Modrinth dependency '{other}'");
                continue;
            }
        };
        if loader.is_some() {
            return Err(Error::Loader(
                "the Modrinth index names more than one mod loader".to_string(),
            ));
        }
        loader = Some(kind);
        version = Some(strip_minecraft_prefix(value, minecraft));
    }
    let version = version.filter(|version| !version.is_empty());
    if loader.is_some() && version.is_none() {
        return Err(Error::Loader(
            "the Modrinth index names a mod loader without a version".to_string(),
        ));
    }
    Ok((loader, version))
}

fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| Error::Manifest(format!("modrinth.index.json is not UTF-8: {error}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text)
        .map_err(|error| Error::Manifest(format!("invalid modrinth.index.json: {error}")))
}

fn file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dependencies(json: &str) -> HashMap<String, String> {
        serde_json::from_str(json).expect("a dependency map")
    }

    #[test]
    fn reads_the_loader_from_the_dependencies() {
        let dependencies = dependencies(r#"{"minecraft": "1.20.1", "fabric-loader": "0.15.0"}"#);
        let (loader, version) = parse_loader(&dependencies).expect("a supported loader");
        assert!(matches!(loader, Some(ModLoaderType::Fabric)));
        assert_eq!(version.as_deref(), Some("0.15.0"));
    }

    #[test]
    fn strips_a_minecraft_prefix_from_a_forge_version() {
        let dependencies = dependencies(r#"{"minecraft": "1.20.1", "forge": "1.20.1-47.2.0"}"#);
        let (loader, version) = parse_loader(&dependencies).expect("a supported loader");
        assert!(matches!(loader, Some(ModLoaderType::Forge)));
        assert_eq!(version.as_deref(), Some("47.2.0"));
    }

    #[test]
    fn allows_a_vanilla_pack() {
        let dependencies = dependencies(r#"{"minecraft": "1.20.1"}"#);
        assert!(parse_loader(&dependencies).expect("no loader").0.is_none());
    }

    #[test]
    fn rejects_two_loaders() {
        let dependencies = dependencies(
            r#"{"minecraft": "1.20.1", "forge": "47.2.0", "fabric-loader": "0.15.0"}"#,
        );
        assert!(parse_loader(&dependencies).is_err());
    }

    #[test]
    fn ignores_a_dependency_it_does_not_know() {
        let dependencies = dependencies(r#"{"minecraft": "1.20.1", "some-future-loader": "1"}"#);
        assert!(parse_loader(&dependencies).expect("no loader").0.is_none());
    }

    #[test]
    fn reads_an_index_and_its_override_trees() {
        let directory =
            std::env::temp_dir().join(format!("conic-modrinth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join("overrides/config")).expect("a temporary directory");
        std::fs::create_dir_all(directory.join("client-overrides/config"))
            .expect("a temporary directory");
        std::fs::write(
            directory.join(MANIFEST),
            r#"{
                "formatVersion": 1,
                "game": "minecraft",
                "versionId": "1.0.0",
                "name": "Test pack",
                "dependencies": { "minecraft": "1.20.1", "quilt-loader": "0.26.0" },
                "files": [
                    {
                        "path": "mods/a.jar",
                        "hashes": { "sha1": "abc", "sha512": "def" },
                        "downloads": ["https://example.invalid/a.jar"],
                        "fileSize": 12
                    }
                ]
            }"#,
        )
        .expect("an index");
        let inspection = crate::inspect(&directory).expect("an inspection");
        assert_eq!(inspection.name, "Test pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        assert_eq!(inspection.loader_version.as_deref(), Some("0.26.0"));
        let crate::Recognized::Pack(crate::pack::Pack::Modrinth(inspection)) =
            &inspection.recognized
        else {
            panic!("a Modrinth pack");
        };
        assert_eq!(inspection.trees.len(), 2);
        std::fs::remove_dir_all(&directory).ok();
    }
}
