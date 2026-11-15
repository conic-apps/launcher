// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The MCBBS pack format: `mcbbs.packmeta` (or a `manifest.json` with an
//! `addons` array) naming the loader addons, the files, and an `overrides/`
//! tree.
//!
//! Its file list mixes two kinds. An `addon` file is a path the pack ships in
//! `overrides/`; when the archive does not carry it, the pack's `fileApi` is
//! where it can be fetched. A `curse` file is a CurseForge project file, so it
//! goes through the same resolution as a CurseForge manifest's entries.

use std::path::Path;

use serde::Deserialize;

use instance::ModLoaderType;

use super::curseforge::{self, CursePair};
use super::{FormatInspection, ResolvedPack, Source, relative_path};
use crate::error::{Error, Result};
use crate::launch::PackLaunch;
use crate::overrides::Overrides;
use crate::{ResolvedFile, UnavailableFile};

const META: &str = "mcbbs.packmeta";
const MANIFEST: &str = "manifest.json";
const OVERRIDES: &str = "overrides";

/// The MCBBS half of an inspection: what is left to fetch once the pack's own
/// `overrides/` tree has been accounted for.
#[derive(Clone)]
pub(crate) struct Inspection {
    overrides: Vec<Overrides>,
    file_api: Option<String>,
    /// `addon` entries the pack does not ship itself.
    pub(super) addon_files: Vec<AddonFile>,
    curse_files: Vec<CursePair>,
}

#[derive(Deserialize)]
struct Pack {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    addons: Vec<Addon>,
    #[serde(default)]
    files: Vec<File>,
    #[serde(default, rename = "fileApi")]
    file_api: Option<String>,
    #[serde(default, rename = "launchInfo")]
    launch_info: LaunchInfo,
    #[serde(default)]
    settings: Settings,
}

#[derive(Deserialize)]
struct Addon {
    #[serde(default)]
    id: String,
    #[serde(default)]
    version: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum File {
    Addon {
        path: String,
        #[serde(default)]
        hash: Option<String>,
    },
    Curse {
        #[serde(rename = "projectID")]
        project_id: i64,
        #[serde(rename = "fileID")]
        file_id: i64,
        #[serde(default, rename = "fileName")]
        file_name: Option<String>,
        #[serde(default)]
        url: Option<String>,
    },
    /// A kind of file this launcher does not know; the pack carries others.
    #[serde(other)]
    Unknown,
}

#[derive(Clone)]
pub(super) struct AddonFile {
    pub(super) path: String,
    hash: Option<String>,
}

#[derive(Deserialize, Default)]
struct LaunchInfo {
    #[serde(default, rename = "minMemory")]
    min_memory: u64,
    #[serde(default, rename = "javaArgument")]
    java_arguments: Vec<String>,
    #[serde(default, rename = "launchArgument")]
    launch_arguments: Vec<String>,
}

#[derive(Deserialize)]
struct Settings {
    #[serde(default = "default_true", rename = "install_mods")]
    install_mods: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { install_mods: true }
    }
}

fn default_true() -> bool {
    true
}

/// Reads an MCBBS manifest into the common inspection.
pub(super) fn inspect(path: &Path, source: &Source) -> Result<crate::Inspection> {
    let names = source.names()?;
    // `mcbbs.packmeta` is the format's own name; an older pack keeps it in
    // `manifest.json`, which is why detection reads the `addons` array there.
    let manifest = if names.iter().any(|name| name == META) {
        META
    } else {
        MANIFEST
    };
    let pack = parse_manifest(&source.read(manifest)?)?;
    let (minecraft, loader_type, loader_version) = read_addons(&pack.addons)?;
    let name = pack.name.trim();
    let name = if name.is_empty() {
        fallback_name(path)
    } else {
        name.to_string()
    };
    log::info!(
        "modpack '{name}': MCBBS {}, Minecraft {minecraft}, {} manifest files",
        pack.version,
        pack.files.len()
    );
    let overrides = if source.has_tree(OVERRIDES)? {
        vec![source.overrides(OVERRIDES)]
    } else {
        Vec::new()
    };
    if !pack.settings.install_mods {
        log::info!("the pack asks for its CurseForge files not to be installed; skipping them");
    }
    let mut addon_files = Vec::new();
    let mut curse_files = Vec::new();
    for file in pack.files {
        match file {
            File::Addon { path, hash } => {
                // A file the archive already carries comes with the overrides.
                if names
                    .iter()
                    .any(|name| name == &format!("{OVERRIDES}/{path}"))
                {
                    continue;
                }
                addon_files.push(AddonFile { path, hash });
            }
            File::Curse {
                project_id,
                file_id,
                file_name,
                url,
            } => {
                if !pack.settings.install_mods {
                    continue;
                }
                curse_files.push(CursePair {
                    project_id,
                    file_id,
                    required: true,
                    fallback: file_name.zip(url),
                });
            }
            File::Unknown => {}
        }
    }
    let file_api = pack
        .file_api
        .map(|api| api.trim().trim_end_matches('/').to_string())
        .filter(|api| !api.is_empty());
    Ok(crate::Inspection {
        name,
        minecraft,
        loader_type,
        loader_version,
        launch: read_launch(&pack.launch_info),
        format: FormatInspection::Mcbbs(Inspection {
            overrides,
            file_api,
            addon_files,
            curse_files,
        }),
    })
}

/// Resolves an MCBBS pack's file list into the common plan.
pub(super) async fn resolve(inspection: Inspection) -> Result<ResolvedPack> {
    let mut files = Vec::new();
    let mut unavailable = Vec::new();
    for addon in &inspection.addon_files {
        let Some(path) = relative_path(&addon.path) else {
            unavailable.push(UnavailableFile {
                file_name: addon.path.clone(),
                reference: addon.path.clone(),
                required: true,
                reason: "the path is not inside the instance".to_string(),
            });
            continue;
        };
        let Some(file_api) = &inspection.file_api else {
            unavailable.push(UnavailableFile {
                file_name: addon.path.clone(),
                reference: addon.path.clone(),
                required: true,
                reason: "the pack neither ships the file nor names a file API".to_string(),
            });
            continue;
        };
        files.push(ResolvedFile {
            path,
            url: format!("{file_api}/{OVERRIDES}/{}", addon.path),
            size_bytes: None,
            checksum: addon
                .hash
                .clone()
                .filter(|hash| !hash.trim().is_empty())
                .map(download::Checksum::Sha1)
                .unwrap_or(download::Checksum::None),
        });
    }

    let (curse_files, curse_unavailable) =
        curseforge::resolve_pairs(&inspection.curse_files).await?;
    files.extend(curse_files);
    unavailable.extend(curse_unavailable);

    Ok(ResolvedPack {
        files,
        unavailable,
        overrides: inspection.overrides,
    })
}

/// The game version and loader, from the manifest's addon list.
fn read_addons(addons: &[Addon]) -> Result<(String, Option<ModLoaderType>, Option<String>)> {
    let minecraft = addons
        .iter()
        .find(|addon| matches!(addon.id.to_ascii_lowercase().as_str(), "game" | "minecraft"))
        .map(|addon| addon.version.trim().to_string())
        .filter(|version| !version.is_empty())
        .ok_or_else(|| Error::Manifest("the MCBBS manifest names no game version".to_string()))?;
    let mut loader_type = None;
    let mut loader_version = None;
    for addon in addons {
        let id = addon.id.to_ascii_lowercase();
        let kind = match id.as_str() {
            "forge" => ModLoaderType::Forge,
            "neoforge" | "neo-forge" => ModLoaderType::Neoforge,
            "fabric" => ModLoaderType::Fabric,
            "quilt" => ModLoaderType::Quilt,
            "optifine" | "liteloader" => {
                log::warn!(
                    "the pack asks for {id} {}, which this launcher does not install",
                    addon.version
                );
                continue;
            }
            "game" | "minecraft" => continue,
            other => {
                log::warn!("ignoring the unknown MCBBS addon '{other}'");
                continue;
            }
        };
        if loader_type.is_some() {
            log::warn!("the pack names more than one mod loader; using the first");
            continue;
        }
        loader_type = Some(kind);
        loader_version = Some(strip_minecraft_prefix(&addon.version, &minecraft));
    }
    Ok((minecraft, loader_type, loader_version))
}

/// Forge ids carry the Minecraft version in front (`1.20.1-47.2.0`).
fn strip_minecraft_prefix(version: &str, minecraft: &str) -> String {
    let prefix = format!("{minecraft}-");
    version.strip_prefix(&prefix).unwrap_or(version).to_string()
}

fn read_launch(info: &LaunchInfo) -> PackLaunch {
    let mut launch = PackLaunch::default();
    let mut jvm_arguments = Vec::new();
    if info.min_memory > 0 {
        // The launcher has no minimum-heap setting of its own, so the pack's
        // `minMemory` travels with the other JVM arguments.
        jvm_arguments.push(format!("-Xms{}M", info.min_memory));
    }
    jvm_arguments.extend(info.java_arguments.iter().cloned());
    launch.set_extra_jvm_args(jvm_arguments);
    launch.set_extra_mc_args(info.launch_arguments.iter().cloned());
    launch
}

fn parse_manifest(bytes: &[u8]) -> Result<Pack> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| Error::Manifest(format!("the MCBBS manifest is not UTF-8: {error}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text)
        .map_err(|error| Error::Manifest(format!("invalid MCBBS manifest: {error}")))
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

    const MANIFEST_JSON: &str = r#"{
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": "MCBBS Pack",
        "version": "1.0.0",
        "author": "someone",
        "fileApi": "https://example.invalid/api/",
        "addons": [
            { "id": "game", "version": "1.20.1" },
            { "id": "forge", "version": "1.20.1-47.2.0" }
        ],
        "files": [
            { "type": "addon", "path": "mods/local.jar", "hash": "abc" },
            { "type": "addon", "path": "mods/remote.jar", "hash": "def" },
            { "type": "curse", "projectID": 238222, "fileID": 4808841, "fileName": "jei.jar", "url": "https://example.invalid/jei.jar" },
            { "type": "unknown-kind", "whatever": true }
        ],
        "launchInfo": {
            "minMemory": 1024,
            "javaArgument": ["-XX:+UseG1GC"],
            "launchArgument": ["--demo"]
        }
    }"#;

    fn pack() -> Pack {
        parse_manifest(MANIFEST_JSON.as_bytes()).expect("a valid manifest")
    }

    #[test]
    fn reads_the_game_and_loader_from_the_addons() {
        let pack = pack();
        let (minecraft, loader, version) = read_addons(&pack.addons).expect("the addons");
        assert_eq!(minecraft, "1.20.1");
        assert!(matches!(loader, Some(ModLoaderType::Forge)));
        assert_eq!(version.as_deref(), Some("47.2.0"));
    }

    #[test]
    fn requires_a_game_addon() {
        assert!(read_addons(&[]).is_err());
    }

    #[test]
    fn reads_the_launch_info() {
        let pack = pack();
        let launch = read_launch(&pack.launch_info);
        let mut applied = instance::InstanceLaunchConfig::default();
        launch.apply(&mut applied);
        assert_eq!(
            applied.extra_jvm_args.as_deref(),
            Some("-Xms1024M -XX:+UseG1GC")
        );
        assert_eq!(applied.extra_mc_args.as_deref(), Some("--demo"));
    }

    #[test]
    fn inspects_a_pack_and_its_missing_addon_files() {
        let directory =
            std::env::temp_dir().join(format!("conic-mcbbs-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join("overrides/mods")).expect("a temporary directory");
        std::fs::write(directory.join("overrides/mods/local.jar"), "jar").expect("a local file");
        std::fs::write(directory.join(META), MANIFEST_JSON).expect("a manifest");
        let inspection = crate::inspect(&directory).expect("an inspection");
        assert_eq!(inspection.name, "MCBBS Pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        let FormatInspection::Mcbbs(inspection) = &inspection.format else {
            panic!("an MCBBS pack");
        };
        // The archive carries `local.jar`; only `remote.jar` is left to fetch.
        assert_eq!(inspection.addon_files.len(), 1);
        assert_eq!(inspection.addon_files[0].path, "mods/remote.jar");
        assert_eq!(inspection.curse_files.len(), 1);
        assert_eq!(inspection.overrides.len(), 1);
        std::fs::remove_dir_all(&directory).ok();
    }
}
