// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The CurseForge pack format.
//!
//! A pack is a `.zip` or extracted directory with a `manifest.json` and an
//! `overrides/` tree. The manifest names the Minecraft version, the loader and
//! `projectID`/`fileID` pairs; this module turns those pairs into downloads
//! through the `curseforge` client and hands the overrides to the common
//! extractor.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;

use instance::ModLoaderType;

use crate::error::{Error, Result};
use crate::overrides::Overrides;
use crate::{ResolvedFile, UnavailableFile};

/// The CurseForge half of an inspection: the unresolved file pairs and the
/// overrides tree the manifest names.
#[derive(Clone)]
pub(crate) struct Inspection {
    files: Vec<ManifestFile>,
    pub(super) overrides: Vec<Overrides>,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    name: String,
    minecraft: ManifestMinecraft,
    #[serde(default)]
    files: Vec<ManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
}

#[derive(Deserialize)]
struct ManifestMinecraft {
    version: String,
    #[serde(default, rename = "modLoaders")]
    mod_loaders: Vec<ManifestLoader>,
}

#[derive(Deserialize)]
struct ManifestLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Clone, Deserialize)]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: i64,
    #[serde(rename = "fileID")]
    file_id: i64,
    #[serde(default = "default_true")]
    required: bool,
}

fn default_overrides() -> String {
    "overrides".to_string()
}

fn default_true() -> bool {
    true
}

/// Reads a CurseForge manifest into the common inspection.
pub(super) fn inspect(path: &Path) -> Result<crate::Inspection> {
    let (manifest, overrides) = read_manifest(path)?;
    let (loader_type, loader_version) = parse_loader(&manifest)?;
    let name = manifest.name.trim();
    let name = if name.is_empty() {
        fallback_name(path)
    } else {
        name.to_string()
    };
    log::info!(
        "modpack '{name}': Minecraft {}, {} manifest files",
        manifest.minecraft.version,
        manifest.files.len()
    );
    Ok(crate::Inspection {
        name,
        minecraft: manifest.minecraft.version,
        loader_type,
        loader_version,
        format: super::FormatInspection::CurseForge(Inspection {
            files: manifest.files,
            overrides,
        }),
    })
}

/// Resolves the CurseForge file pairs into the common plan.
pub(super) async fn resolve(inspection: Inspection) -> Result<super::ResolvedPack> {
    let (files, unavailable) = resolve_files(&inspection.files).await?;
    Ok(super::ResolvedPack {
        files,
        unavailable,
        overrides: inspection.overrides,
    })
}

/// Reads the manifest from a dropped zip or directory.
fn read_manifest(path: &Path) -> Result<(Manifest, Vec<Overrides>)> {
    if path.is_dir() {
        let base = find_manifest_dir(path)?;
        let manifest_path = base.join("manifest.json");
        let bytes = std::fs::read(&manifest_path).map_err(|source| Error::Read {
            path: manifest_path.display().to_string(),
            source,
        })?;
        let manifest = parse_manifest(&bytes)?;
        let overrides = directory_overrides(&base, &manifest.overrides);
        Ok((manifest, overrides))
    } else {
        let (manifest, base) = read_archive_manifest(path)?;
        let overrides = archive_overrides(path, &base, &manifest.overrides);
        Ok((manifest, overrides))
    }
}

fn directory_overrides(base: &Path, overrides: &str) -> Vec<Overrides> {
    if overrides.trim().is_empty() {
        Vec::new()
    } else {
        vec![Overrides::Directory {
            base: base.to_path_buf(),
            relative: overrides.to_string(),
        }]
    }
}

fn archive_overrides(path: &Path, base: &str, overrides: &str) -> Vec<Overrides> {
    if overrides.trim().is_empty() {
        Vec::new()
    } else {
        vec![Overrides::Archive {
            path: path.to_path_buf(),
            prefix: format!("{base}{}/", overrides.trim_matches('/')),
        }]
    }
}

fn find_manifest_dir(path: &Path) -> Result<PathBuf> {
    if path.join("manifest.json").is_file() {
        return Ok(path.to_path_buf());
    }
    let mut found = Vec::new();
    let entries = std::fs::read_dir(path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        let candidate = entry.path();
        if candidate.is_dir() && candidate.join("manifest.json").is_file() {
            found.push(candidate);
        }
    }
    match found.len() {
        0 => Err(Error::Manifest(format!(
            "{} does not contain a CurseForge manifest.json",
            path.display()
        ))),
        1 => Ok(found.remove(0)),
        _ => Err(Error::Manifest(format!(
            "{} contains more than one CurseForge modpack; drop the pack folder itself",
            path.display()
        ))),
    }
}

fn read_archive_manifest(path: &Path) -> Result<(Manifest, String)> {
    let file = std::fs::File::open(path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|source| Error::Zip {
        path: path.display().to_string(),
        source,
    })?;
    let names: Vec<String> = (0..archive.len())
        .filter_map(|index| archive.name_for_index(index).map(str::to_owned))
        .collect();
    let entry = choose_manifest(&names)?;
    let mut file = archive.by_name(&entry).map_err(|source| Error::Zip {
        path: path.display().to_string(),
        source,
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|source| Error::Read {
        path: format!("{} ({entry})", path.display()),
        source,
    })?;
    let manifest = parse_manifest(&bytes)?;
    let base = entry
        .strip_suffix("manifest.json")
        .unwrap_or_default()
        .to_string();
    Ok((manifest, base))
}

/// The archive entry holding `manifest.json`: root first, then the shallowest
/// nested one. Two manifests at the same depth are ambiguous.
fn choose_manifest(names: &[String]) -> Result<String> {
    let mut candidates: Vec<(usize, &String)> = names
        .iter()
        .filter(|name| name.rsplit('/').next() == Some("manifest.json"))
        .map(|name| (name.matches('/').count(), name))
        .collect();
    candidates.sort_by_key(|(depth, _)| *depth);
    let Some((depth, first)) = candidates.first() else {
        return Err(Error::Manifest(
            "the archive does not contain a CurseForge manifest.json".to_string(),
        ));
    };
    if candidates
        .iter()
        .filter(|(candidate_depth, _)| candidate_depth == depth)
        .count()
        > 1
    {
        return Err(Error::Manifest(
            "the archive contains more than one CurseForge manifest.json".to_string(),
        ));
    }
    Ok((*first).clone())
}

fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| Error::Manifest(format!("manifest.json is not UTF-8: {error}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text)
        .map_err(|error| Error::Manifest(format!("invalid manifest.json: {error}")))
}

fn parse_loader(manifest: &Manifest) -> Result<(Option<ModLoaderType>, Option<String>)> {
    let Some(loader) = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|loader| loader.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
    else {
        return Ok((None, None));
    };
    let (kind, version) = loader
        .id
        .split_once('-')
        .ok_or_else(|| Error::Loader(format!("unsupported mod loader id '{}'", loader.id)))?;
    let kind = match kind.to_ascii_lowercase().as_str() {
        "forge" => ModLoaderType::Forge,
        "neoforge" => ModLoaderType::Neoforge,
        "fabric" => ModLoaderType::Fabric,
        "quilt" => ModLoaderType::Quilt,
        other => {
            return Err(Error::Loader(format!(
                "unsupported mod loader '{other}' in the manifest"
            )));
        }
    };
    let minecraft_prefix = format!("{}-", manifest.minecraft.version);
    let version = version.strip_prefix(&minecraft_prefix).unwrap_or(version);
    if version.is_empty() {
        return Err(Error::Loader(format!(
            "the manifest's loader id '{}' has no version",
            loader.id
        )));
    }
    Ok((Some(kind), Some(version.to_string())))
}

/// Resolves every `projectID`/`fileID` pair into a download.
///
/// The batch endpoint is tried first; a mirror that does not implement it, or an
/// id it did not return, falls back to the per-file endpoint.
async fn resolve_files(
    files: &[ManifestFile],
) -> Result<(Vec<ResolvedFile>, Vec<UnavailableFile>)> {
    if files.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut metadata: HashMap<i64, Value> = HashMap::new();
    let mut last_error = None;
    for chunk in files.chunks(100) {
        let ids: Vec<i64> = chunk.iter().map(|file| file.file_id).collect();
        match curseforge::get_files(&serde_json::json!({ "fileIds": ids })).await {
            Ok(value) => {
                if let Some(data) = value.get("data").and_then(Value::as_array) {
                    for file in data {
                        if let Some(id) = file.get("id").and_then(Value::as_i64) {
                            metadata.insert(id, file.clone());
                        }
                    }
                }
            }
            Err(error) => {
                last_error = Some(error.to_string());
                log::warn!("CurseForge batch file lookup failed: {error}");
            }
        }
    }

    let missing: Vec<ManifestFile> = files
        .iter()
        .filter(|file| !metadata.contains_key(&file.file_id))
        .cloned()
        .collect();
    if !missing.is_empty() {
        let results = futures::stream::iter(missing.into_iter().map(|file| async move {
            let result = curseforge::get_mod_file(file.project_id, file.file_id).await;
            (file, result)
        }))
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;
        for (file, result) in results {
            match result {
                Ok(value) => {
                    if let Some(data) = value.get("data").filter(|data| !data.is_null()) {
                        metadata.insert(file.file_id, data.clone());
                    }
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                    log::warn!(
                        "CurseForge lookup for project {} file {} failed: {error}",
                        file.project_id,
                        file.file_id
                    );
                }
            }
        }
    }
    if metadata.is_empty()
        && let Some(error) = last_error
    {
        return Err(Error::Source(error));
    }

    let needs_url: Vec<(i64, i64)> = files
        .iter()
        .filter_map(|file| {
            let api_file = metadata.get(&file.file_id)?;
            let url = api_file
                .get("downloadUrl")
                .and_then(Value::as_str)
                .unwrap_or_default();
            url.is_empty().then_some((file.project_id, file.file_id))
        })
        .collect();
    let mut fallback_urls: HashMap<i64, Option<String>> = HashMap::new();
    if !needs_url.is_empty() {
        let results = futures::stream::iter(needs_url.into_iter().map(
            |(project_id, file_id)| async move {
                let result = curseforge::get_mod_file_download_url(project_id, file_id).await;
                (file_id, result)
            },
        ))
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;
        for (file_id, result) in results {
            let url = result.ok().and_then(|value| {
                value
                    .get("data")
                    .and_then(Value::as_str)
                    .filter(|url| !url.is_empty())
                    .map(str::to_owned)
            });
            fallback_urls.insert(file_id, url);
        }
    }

    let mut resolved = Vec::new();
    let mut unavailable = Vec::new();
    let mut names = HashSet::new();
    for file in files {
        let Some(api_file) = metadata.get(&file.file_id) else {
            unavailable.push(UnavailableFile {
                file_name: format!("project {} / file {}", file.project_id, file.file_id),
                reference: format!("project {} / file {}", file.project_id, file.file_id),
                required: file.required,
                reason: "the file is no longer available on CurseForge".to_string(),
            });
            continue;
        };
        let raw_name = api_file
            .get("fileName")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(file_name) = valid_file_name(raw_name) else {
            unavailable.push(UnavailableFile {
                file_name: format!("project {} / file {}", file.project_id, file.file_id),
                reference: format!("project {} / file {}", file.project_id, file.file_id),
                required: file.required,
                reason: format!("CurseForge returned an unusable file name ({raw_name:?})"),
            });
            continue;
        };
        if api_file
            .get("isAvailable")
            .and_then(Value::as_bool)
            .is_some_and(|available| !available)
        {
            unavailable.push(UnavailableFile {
                file_name,
                reference: format!("project {} / file {}", file.project_id, file.file_id),
                required: file.required,
                reason: "the file is marked unavailable on CurseForge".to_string(),
            });
            continue;
        }
        if !names.insert(file_name.to_ascii_lowercase()) {
            unavailable.push(UnavailableFile {
                file_name,
                reference: format!("project {} / file {}", file.project_id, file.file_id),
                required: file.required,
                reason: "another manifest entry already uses this file name".to_string(),
            });
            continue;
        }
        let url = api_file
            .get("downloadUrl")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_owned)
            .or_else(|| fallback_urls.get(&file.file_id).cloned().flatten());
        let Some(url) = url else {
            unavailable.push(UnavailableFile {
                file_name,
                reference: format!("project {} / file {}", file.project_id, file.file_id),
                required: file.required,
                reason: "the author disabled third-party downloads".to_string(),
            });
            continue;
        };
        resolved.push(ResolvedFile {
            path: PathBuf::from("mods").join(file_name),
            url,
            size_bytes: api_file.get("fileLength").and_then(Value::as_u64),
            sha1: api_file
                .get("hashes")
                .and_then(Value::as_array)
                .and_then(|hashes| {
                    hashes
                        .iter()
                        .find(|hash| hash.get("algo").and_then(Value::as_i64) == Some(1))
                        .and_then(|hash| hash.get("value").and_then(Value::as_str))
                        .map(str::to_owned)
                }),
        });
    }
    Ok((resolved, unavailable))
}

/// The API's `fileName` is written into `mods/`, so it must be one plain name.
fn valid_file_name(raw: &str) -> Option<String> {
    let name = raw.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return None;
    }
    let path = Path::new(name);
    if path.components().count() != 1 {
        return None;
    }
    Some(name.to_string())
}

fn fallback_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Modpack".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> Manifest {
        parse_manifest(json.as_bytes()).expect("a valid manifest")
    }

    #[test]
    fn reads_the_pack_and_its_loader() {
        let manifest = manifest(
            r#"{
                "name": "DeceasedCraft_CB",
                "minecraft": {
                    "version": "1.20.1",
                    "modLoaders": [{ "id": "forge-47.4.0", "primary": true }]
                },
                "files": [
                    { "projectID": 280510, "fileID": 7612291, "required": true }
                ]
            }"#,
        );
        assert_eq!(manifest.name, "DeceasedCraft_CB");
        assert_eq!(manifest.minecraft.version, "1.20.1");
        assert_eq!(manifest.overrides, "overrides");
        assert_eq!(manifest.files.len(), 1);
        let (loader, version) = parse_loader(&manifest).expect("a supported loader");
        assert!(matches!(loader, Some(ModLoaderType::Forge)));
        assert_eq!(version.as_deref(), Some("47.4.0"));
    }

    #[test]
    fn strips_a_minecraft_prefix_from_a_legacy_forge_id() {
        let manifest = manifest(
            r#"{
                "minecraft": {
                    "version": "1.12.2",
                    "modLoaders": [{ "id": "forge-1.12.2-14.23.5.2859", "primary": true }]
                }
            }"#,
        );
        let (_, version) = parse_loader(&manifest).expect("a supported loader");
        assert_eq!(version.as_deref(), Some("14.23.5.2859"));
    }

    #[test]
    fn allows_a_vanilla_pack() {
        let manifest = manifest(r#"{"minecraft": {"version": "1.20.1"}}"#);
        assert!(parse_loader(&manifest).expect("no loader").0.is_none());
    }

    #[test]
    fn rejects_an_unknown_loader() {
        let manifest = manifest(
            r#"{
                "minecraft": {
                    "version": "1.20.1",
                    "modLoaders": [{ "id": "liteloader-1.0", "primary": true }]
                }
            }"#,
        );
        assert!(parse_loader(&manifest).is_err());
    }

    #[test]
    fn picks_the_root_manifest_over_a_nested_one() {
        let names = vec![
            "pack/manifest.json".to_string(),
            "manifest.json".to_string(),
        ];
        assert_eq!(
            choose_manifest(&names).expect("a manifest"),
            "manifest.json"
        );
    }

    #[test]
    fn picks_a_single_nested_manifest() {
        let names = vec![
            "pack/manifest.json".to_string(),
            "pack/overrides/config/a.toml".to_string(),
        ];
        assert_eq!(
            choose_manifest(&names).expect("a manifest"),
            "pack/manifest.json"
        );
    }

    #[test]
    fn rejects_two_manifests_at_the_same_depth() {
        let names = vec!["a/manifest.json".to_string(), "b/manifest.json".to_string()];
        assert!(choose_manifest(&names).is_err());
    }

    #[test]
    fn rejects_path_like_file_names() {
        assert_eq!(valid_file_name("mod.jar").as_deref(), Some("mod.jar"));
        assert_eq!(valid_file_name("../mod.jar"), None);
        assert_eq!(valid_file_name("a/b.jar"), None);
        assert_eq!(valid_file_name("a\\b.jar"), None);
        assert_eq!(valid_file_name(""), None);
    }

    #[test]
    fn inspects_an_extracted_pack_without_touching_the_network() {
        let directory =
            std::env::temp_dir().join(format!("conic-modpack-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        std::fs::write(
            directory.join("manifest.json"),
            r#"{
                "name": "Empty pack",
                "minecraft": { "version": "1.20.1" },
                "files": []
            }"#,
        )
        .expect("a manifest");
        let inspection = crate::inspect(&directory).expect("an inspection");
        assert_eq!(inspection.name, "Empty pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        std::fs::remove_dir_all(&directory).ok();
    }
}
