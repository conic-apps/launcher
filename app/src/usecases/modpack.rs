// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CurseForge modpack import.
//!
//! A pack is either a `.zip` or an extracted directory containing a CurseForge
//! `manifest.json` and an `overrides/` tree. Importing creates a normal instance
//! under `instances/`: the pack's Minecraft version and loader go into
//! `instance.toml`, the manifest's `files` become the instance's `mods/`, and
//! `overrides/` is copied over the instance root after the game itself is
//! installed, so the pack's configs and `options.txt` win.
//!
//! The manifest only carries `projectID`/`fileID` pairs, not URLs, so each file
//! has to be resolved through the `curseforge` client. A file whose author has
//! disabled third-party downloads has no URL; [`Plan::unavailable`] carries those
//! so the caller can list them and ask before continuing without them.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;

use config::Config;
use download::progress::DownloadState;
use instance::{InstanceConfig, InstanceLaunchConfig, InstanceRuntime, ModLoaderType};
use storage::LOCATIONS;

use super::launch::{LaunchSink, LaunchUpdate};
use crate::support::fs;

/// What a dropped pack is, before its file list is resolved.
#[derive(Clone)]
pub(crate) struct Inspection {
    /// The pack's display name, which seeds the instance name.
    pub(crate) name: String,
    pub(crate) minecraft: String,
    pub(crate) loader_type: Option<ModLoaderType>,
    pub(crate) loader_version: Option<String>,
    source: Source,
    files: Vec<ManifestFile>,
}

/// A fully resolved pack, ready to install.
pub(crate) struct Plan {
    /// The pack's display name, which seeds the instance id.
    pub(crate) name: String,
    /// The files that could not be resolved, in manifest order.
    pub(crate) unavailable: Vec<UnavailableFile>,
    minecraft: String,
    loader_type: Option<ModLoaderType>,
    loader_version: Option<String>,
    source: Source,
    files: Vec<ResolvedFile>,
}

/// A manifest file that could not be turned into a download.
pub(crate) struct UnavailableFile {
    /// The file name when the API knew it, otherwise a `project/file` label.
    pub(crate) file_name: String,
    pub(crate) project_id: i64,
    pub(crate) file_id: i64,
    pub(crate) required: bool,
    pub(crate) reason: String,
}

struct ResolvedFile {
    file_name: String,
    url: String,
    size_bytes: Option<u64>,
    sha1: Option<String>,
}

/// Where the pack's files live, and where its overrides are inside them.
#[derive(Clone)]
enum Source {
    Directory {
        base: PathBuf,
        overrides: String,
    },
    Archive {
        path: PathBuf,
        /// The archive entry prefix that contains `manifest.json`; empty at the
        /// archive root, `pack/` for a pack zipped inside a folder.
        base: String,
        overrides: String,
    },
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

/// Reads the manifest of a dropped pack. No network, no instance.
pub(crate) fn inspect(path: &Path) -> Result<Inspection, String> {
    let (manifest, source) = read_manifest(path)?;
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
    Ok(Inspection {
        name,
        minecraft: manifest.minecraft.version,
        loader_type,
        loader_version,
        source,
        files: manifest.files,
    })
}

/// Resolves the manifest's CurseForge file pairs into downloads.
pub(crate) async fn resolve(inspection: Inspection) -> Result<Plan, String> {
    let (files, unavailable) = resolve_files(&inspection.files).await?;
    log::info!(
        "modpack '{}': {} downloadable files, {} unavailable",
        inspection.name,
        files.len(),
        unavailable.len()
    );
    Ok(Plan {
        name: inspection.name,
        unavailable,
        minecraft: inspection.minecraft,
        loader_type: inspection.loader_type,
        loader_version: inspection.loader_version,
        source: inspection.source,
        files,
    })
}

/// Creates the instance for a resolved pack.
///
/// The instance is written before the install starts so the launch screen can
/// show its name and runtime while the download runs. A cancelled install
/// therefore leaves an uninstalled instance behind, exactly like cancelling a
/// first launch does.
pub(crate) async fn create_instance(plan: &Plan, name: &str) -> Result<String, String> {
    let name = name.trim();
    let name = if name.is_empty() {
        plan.name.as_str()
    } else {
        name
    };
    let id = instance::unique_id(name)
        .await
        .map_err(|error| format!("could not pick an instance id: {error}"))?;
    let instance_config = InstanceConfig {
        name: name.to_string(),
        runtime: InstanceRuntime {
            minecraft: plan.minecraft.clone(),
            mod_loader_type: plan.loader_type.clone(),
            mod_loader_version: plan.loader_version.clone(),
        },
        launch_config: InstanceLaunchConfig {
            enable_instance_specific_settings: false,
            ..Default::default()
        },
        ..Default::default()
    };
    if let Err(error) = instance::create_instance(instance_config, Some(&id)).await {
        let _ = instance::delete_instance(&id).await;
        return Err(format!("could not create the instance: {error}"));
    }
    Ok(id)
}

/// Installs a created instance's game, loader, files and overrides.
///
/// Reports through the launch screen's port so the same progress panel and
/// cancel button drive the import.
pub(crate) async fn run_install(id: String, plan: Plan, config: Config, out: LaunchSink) {
    let Some(instance) = instance::get_instance_by_id(&id).await else {
        out(LaunchUpdate::Error(
            "the new instance could not be read back".to_string(),
        ));
        return;
    };
    for file in &plan.unavailable {
        log::info!("skipping modpack file {}: {}", file.file_name, file.reason);
    }
    log::info!(
        "installing Minecraft {} for instance '{id}'",
        instance.config.runtime.minecraft
    );
    // Once `install::install` finishes it writes `.install.lock`. Anything that
    // fails or is cancelled after that must take the lock back, or the launch
    // page would skip the repair pass on a half-filled instance.
    let mut lock = InstallLockGuard {
        id: id.clone(),
        armed: true,
    };

    let download_config = config.download.clone();
    let install_out = out.clone();
    let sink: install::InstallSink =
        Arc::new(move |progress| install_out(LaunchUpdate::Install(progress)));
    if let Err(error) = install::install(config, instance, sink).await {
        out(LaunchUpdate::Error(format!(
            "could not install Minecraft and the loader: {error}"
        )));
        return;
    }

    let mods_dir = LOCATIONS.instances.get_instance_root(&id).join("mods");
    let downloads: Vec<download::DownloadTask> = plan
        .files
        .iter()
        .map(|file| download::DownloadTask {
            url: file.url.clone(),
            file: mods_dir.join(&file.file_name),
            size_bytes: file.size_bytes,
            checksum: file
                .sha1
                .clone()
                .map(download::Checksum::Sha1)
                .unwrap_or(download::Checksum::None),
            task_type: download::DownloadTaskType::CurseforgeMod,
        })
        .collect();
    if !downloads.is_empty() {
        log::info!("downloading {} modpack files", downloads.len());
        let state = DownloadState::default();
        let progress_out = out.clone();
        let result = download::progress::watch(
            &state,
            move |snapshot| progress_out(LaunchUpdate::Modpack(snapshot)),
            download::download_concurrent(downloads, &state, download_config),
        )
        .await;
        if let Err(error) = result {
            out(LaunchUpdate::Error(format!(
                "could not download the modpack's files: {error}"
            )));
            return;
        }
    }

    if let Err(error) =
        extract_overrides(&plan.source, &LOCATIONS.instances.get_instance_root(&id)).await
    {
        out(LaunchUpdate::Error(error));
        return;
    }

    lock.disarm();
    out(LaunchUpdate::Finished {
        quit_app: false,
        pause_music: false,
    });
}

/// Removes `.install.lock` when an install is dropped before it finishes.
struct InstallLockGuard {
    id: String,
    armed: bool,
}

impl InstallLockGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for InstallLockGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let id = self.id.clone();
        tokio::spawn(async move {
            let _ = instance::remove_install_lock(&id).await;
        });
    }
}

/// Reads the manifest from a dropped zip or directory.
fn read_manifest(path: &Path) -> Result<(Manifest, Source), String> {
    if path.is_dir() {
        let base = find_manifest_dir(path)?;
        let bytes = std::fs::read(base.join("manifest.json"))
            .map_err(|error| format!("could not read {}: {error}", base.display()))?;
        let manifest = parse_manifest(&bytes)?;
        let overrides = manifest.overrides.clone();
        Ok((manifest, Source::Directory { base, overrides }))
    } else {
        let (manifest, base) = read_archive_manifest(path)?;
        let overrides = manifest.overrides.clone();
        Ok((
            manifest,
            Source::Archive {
                path: path.to_path_buf(),
                base,
                overrides,
            },
        ))
    }
}

fn find_manifest_dir(path: &Path) -> Result<PathBuf, String> {
    if path.join("manifest.json").is_file() {
        return Ok(path.to_path_buf());
    }
    let mut found = Vec::new();
    let entries = std::fs::read_dir(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let candidate = entry.path();
        if candidate.is_dir() && candidate.join("manifest.json").is_file() {
            found.push(candidate);
        }
    }
    match found.len() {
        0 => Err(format!(
            "{} does not contain a CurseForge manifest.json",
            path.display()
        )),
        1 => Ok(found.remove(0)),
        _ => Err(format!(
            "{} contains more than one CurseForge modpack; drop the pack folder itself",
            path.display()
        )),
    }
}

fn read_archive_manifest(path: &Path) -> Result<(Manifest, String), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("{} is not a readable zip archive: {error}", path.display()))?;
    let names: Vec<String> = (0..archive.len())
        .filter_map(|index| archive.name_for_index(index).map(str::to_owned))
        .collect();
    if names
        .iter()
        .any(|name| name.rsplit('/').next() == Some("modrinth.index.json"))
    {
        return Err(
            "this is a Modrinth .mrpack; only CurseForge packs are supported for now".to_string(),
        );
    }
    let entry = choose_manifest(&names)?;
    let mut file = archive
        .by_name(&entry)
        .map_err(|error| format!("could not read {entry} in {}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("could not read {entry} in {}: {error}", path.display()))?;
    let manifest = parse_manifest(&bytes)?;
    let base = entry
        .strip_suffix("manifest.json")
        .unwrap_or_default()
        .to_string();
    Ok((manifest, base))
}

/// The archive entry holding `manifest.json`: root first, then the shallowest
/// nested one. Two manifests at the same depth are ambiguous.
fn choose_manifest(names: &[String]) -> Result<String, String> {
    let mut candidates: Vec<(usize, &String)> = names
        .iter()
        .filter(|name| name.rsplit('/').next() == Some("manifest.json"))
        .map(|name| (name.matches('/').count(), name))
        .collect();
    candidates.sort_by_key(|(depth, _)| *depth);
    let Some((depth, first)) = candidates.first() else {
        return Err("the archive does not contain a CurseForge manifest.json".to_string());
    };
    if candidates
        .iter()
        .filter(|(candidate_depth, _)| candidate_depth == depth)
        .count()
        > 1
    {
        return Err("the archive contains more than one CurseForge manifest.json".to_string());
    }
    Ok((*first).clone())
}

fn parse_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("manifest.json is not UTF-8: {error}"))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text).map_err(|error| format!("invalid manifest.json: {error}"))
}

fn parse_loader(manifest: &Manifest) -> Result<(Option<ModLoaderType>, Option<String>), String> {
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
        .ok_or_else(|| format!("unsupported mod loader id '{}'", loader.id))?;
    let kind = match kind.to_ascii_lowercase().as_str() {
        "forge" => ModLoaderType::Forge,
        "neoforge" => ModLoaderType::Neoforge,
        "fabric" => ModLoaderType::Fabric,
        "quilt" => ModLoaderType::Quilt,
        other => return Err(format!("unsupported mod loader '{other}' in the manifest")),
    };
    let minecraft_prefix = format!("{}-", manifest.minecraft.version);
    let version = version.strip_prefix(&minecraft_prefix).unwrap_or(version);
    if version.is_empty() {
        return Err(format!(
            "the manifest's loader id '{}' has no version",
            loader.id
        ));
    }
    Ok((Some(kind), Some(version.to_string())))
}

/// Resolves every `projectID`/`fileID` pair into a download.
///
/// The batch endpoint is tried first; a mirror that does not implement it, or an
/// id it did not return, falls back to the per-file endpoint.
async fn resolve_files(
    files: &[ManifestFile],
) -> Result<(Vec<ResolvedFile>, Vec<UnavailableFile>), String> {
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
        return Err(format!("could not resolve the CurseForge files: {error}"));
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
                project_id: file.project_id,
                file_id: file.file_id,
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
                project_id: file.project_id,
                file_id: file.file_id,
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
                project_id: file.project_id,
                file_id: file.file_id,
                required: file.required,
                reason: "the file is marked unavailable on CurseForge".to_string(),
            });
            continue;
        }
        if !names.insert(file_name.to_ascii_lowercase()) {
            unavailable.push(UnavailableFile {
                file_name,
                project_id: file.project_id,
                file_id: file.file_id,
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
                project_id: file.project_id,
                file_id: file.file_id,
                required: file.required,
                reason: "the author disabled third-party downloads".to_string(),
            });
            continue;
        };
        resolved.push(ResolvedFile {
            file_name,
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

async fn extract_overrides(source: &Source, instance_root: &Path) -> Result<(), String> {
    match source {
        Source::Directory { base, overrides } => {
            if overrides.trim().is_empty() {
                return Ok(());
            }
            let from = base.join(overrides);
            if !from.is_dir() {
                log::warn!("the modpack has no {} directory", from.display());
                return Ok(());
            }
            let to = instance_root.to_path_buf();
            tokio::task::spawn_blocking(move || fs::copy_recursively(&from, &to))
                .await
                .map_err(|error| format!("copying the modpack overrides failed: {error}"))?
                .map_err(|error| format!("copying the modpack overrides failed: {error}"))
        }
        Source::Archive {
            path,
            base,
            overrides,
        } => {
            if overrides.trim().is_empty() {
                return Ok(());
            }
            let path = path.clone();
            let prefix = format!("{base}{}/", overrides.trim_matches('/'));
            let to = instance_root.to_path_buf();
            tokio::task::spawn_blocking(move || extract_archive_overrides(&path, &prefix, &to))
                .await
                .map_err(|error| format!("extracting the modpack overrides failed: {error}"))?
        }
    }
}

fn extract_archive_overrides(path: &Path, prefix: &str, to: &Path) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("{} is not a readable zip archive: {error}", path.display()))?;
    let mut extracted = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        // `enclosed_name` rejects `..` and absolute entries, so a pack cannot
        // write outside the instance root.
        let Some(name) = entry.enclosed_name() else {
            log::warn!(
                "skipping {} in {}: it is not an enclosed path",
                entry.name(),
                path.display()
            );
            continue;
        };
        let name = name.to_string_lossy().replace('\\', "/");
        let Some(relative) = name.strip_prefix(prefix) else {
            continue;
        };
        if relative.is_empty() {
            continue;
        }
        let destination = to.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination)
                .map_err(|error| format!("could not create {}: {error}", destination.display()))?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        let mut output = std::fs::File::create(&destination)
            .map_err(|error| format!("could not create {}: {error}", destination.display()))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| format!("could not write {}: {error}", destination.display()))?;
        extracted += 1;
    }
    log::info!("extracted {extracted} modpack override files");
    Ok(())
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
    fn sanitizes_instance_ids() {
        assert_eq!(instance::sanitize_id("DUEL OF THE END"), "DUEL OF THE END");
        assert_eq!(instance::sanitize_id("a/b:c"), "a_b_c");
        assert_eq!(instance::sanitize_id("con"), "_con");
        assert_eq!(instance::sanitize_id("   "), "Instance");
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
        let inspection = inspect(&directory).expect("an inspection");
        assert_eq!(inspection.name, "Empty pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        assert!(inspection.files.is_empty());
        std::fs::remove_dir_all(&directory).ok();
    }

    #[tokio::test]
    async fn copies_directory_overrides_into_the_instance_root() {
        let source =
            std::env::temp_dir().join(format!("conic-modpack-source-{}", uuid::Uuid::new_v4()));
        let destination = std::env::temp_dir().join(format!(
            "conic-modpack-destination-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(source.join("overrides/config")).expect("the pack tree");
        std::fs::write(source.join("overrides/config/a.toml"), "value = true\n")
            .expect("an override");
        extract_overrides(
            &Source::Directory {
                base: source.clone(),
                overrides: "overrides".to_string(),
            },
            &destination,
        )
        .await
        .expect("the overrides");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&destination).ok();
    }

    #[tokio::test]
    async fn reads_and_extracts_a_zip_with_a_nested_root() {
        use std::io::Write as _;

        let archive = std::env::temp_dir().join(format!(
            "conic-modpack-archive-{}.zip",
            uuid::Uuid::new_v4()
        ));
        {
            let file = std::fs::File::create(&archive).expect("a zip file");
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("pack/manifest.json", options)
                .expect("the manifest entry");
            zip.write_all(
                br#"{
                    "name": "Zipped pack",
                    "minecraft": { "version": "1.20.1" },
                    "files": []
                }"#,
            )
            .expect("the manifest");
            zip.start_file("pack/overrides/config/a.toml", options)
                .expect("the override entry");
            zip.write_all(b"value = true\n").expect("the override");
            zip.finish().expect("a finished zip");
        }

        let inspection = inspect(&archive).expect("an inspection");
        assert_eq!(inspection.name, "Zipped pack");
        let destination =
            std::env::temp_dir().join(format!("conic-modpack-unpacked-{}", uuid::Uuid::new_v4()));
        extract_overrides(&inspection.source, &destination)
            .await
            .expect("the overrides");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        std::fs::remove_file(&archive).ok();
        std::fs::remove_dir_all(&destination).ok();
    }
}
