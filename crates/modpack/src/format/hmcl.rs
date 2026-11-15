// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The HMCL pack format: an old-style HMCL export with a `modpack.json`
//! describing the pack, a `minecraft/pack.json` version manifest, and the game
//! files under `minecraft/`.
//!
//! Everything the pack needs is in the archive, so nothing is downloaded. The
//! loader is not named anywhere directly: `pack.json` carries it as a library
//! or as an `--fml.*Version` launch argument, the same way HMCL reads it.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use instance::ModLoaderType;

use super::{FormatInspection, ResolvedPack, Source};
use crate::error::{Error, Result};
use crate::launch::PackLaunch;
use crate::overrides::Overrides;

const MANIFEST: &str = "modpack.json";
/// The pack's game directory, and the version manifest HMCL writes in it.
const GAME_DIRECTORY: &str = "minecraft";
const VERSION_MANIFEST: &str = "minecraft/pack.json";
/// Content of the game directory that this launcher installs itself.
const OWN_CONTENT: &[&str] = &["pack.json", "versions", "libraries", "assets"];

#[derive(Clone)]
pub(crate) struct Inspection {
    overrides: Vec<Overrides>,
}

#[derive(Deserialize)]
struct Modpack {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "gameVersion")]
    game_version: String,
}

#[derive(Deserialize)]
struct Pack {
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(default)]
    arguments: Option<Arguments>,
    #[serde(default, rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
}

#[derive(Deserialize)]
struct Library {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct Arguments {
    #[serde(default)]
    game: Vec<Value>,
}

/// Reads an HMCL pack into the common inspection.
pub(super) fn inspect(path: &Path, source: &Source) -> Result<crate::Inspection> {
    let modpack: Modpack = parse(&source.read(MANIFEST)?, MANIFEST)?;
    let names = source.names()?;
    let pack: Option<Pack> = if names.iter().any(|name| name == VERSION_MANIFEST) {
        Some(parse(&source.read(VERSION_MANIFEST)?, VERSION_MANIFEST)?)
    } else {
        log::warn!("the HMCL pack has no minecraft/pack.json; its loader cannot be read");
        None
    };
    let minecraft = modpack.game_version.trim().to_string();
    if minecraft.is_empty() {
        return Err(Error::Manifest(
            "the HMCL pack names no game version".to_string(),
        ));
    }
    let (loader_type, loader_version) = match pack
        .as_ref()
        .and_then(|pack| detect_loader(pack, &minecraft))
    {
        Some((kind, version)) => (Some(kind), Some(version)),
        None => {
            if pack.is_some() {
                log::info!(
                    "the HMCL pack carries no loader this launcher recognises; installing it as \
                     vanilla"
                );
            }
            (None, None)
        }
    };
    let name = modpack.name.trim();
    let name = if name.is_empty() {
        fallback_name(path)
    } else {
        name.to_string()
    };
    log::info!("modpack '{name}': HMCL, Minecraft {minecraft}");
    let overrides = if source.has_tree(GAME_DIRECTORY)? {
        vec![source.overrides(GAME_DIRECTORY).skipping(OWN_CONTENT)]
    } else {
        Vec::new()
    };
    Ok(crate::Inspection {
        name,
        minecraft,
        loader_type,
        loader_version,
        launch: PackLaunch::default(),
        format: FormatInspection::Hmcl(Inspection { overrides }),
    })
}

/// Resolves an HMCL pack into the common plan; it downloads nothing.
pub(super) fn resolve(inspection: Inspection) -> Result<ResolvedPack> {
    Ok(ResolvedPack {
        files: Vec::new(),
        unavailable: Vec::new(),
        overrides: inspection.overrides,
    })
}

/// The loader `pack.json` was built for.
///
/// The launch arguments are checked first: they carry the loader version
/// verbatim, while a library version can be prefixed with the Minecraft one.
fn detect_loader(pack: &Pack, minecraft: &str) -> Option<(ModLoaderType, String)> {
    let arguments = game_arguments(pack);
    for (flag, kind) in [
        ("--fml.neoForgeVersion", ModLoaderType::Neoforge),
        ("--fml.forgeVersion", ModLoaderType::Forge),
    ] {
        if let Some(version) = argument_value(&arguments, flag) {
            return Some((kind, version.to_string()));
        }
    }
    for library in &pack.libraries {
        let Some((group, artifact, version)) = split_library(&library.name) else {
            continue;
        };
        let kind = match (group, artifact) {
            ("net.fabricmc", "fabric-loader") => ModLoaderType::Fabric,
            ("org.quiltmc", "quilt-loader") => ModLoaderType::Quilt,
            ("net.neoforged", "neoforge") => ModLoaderType::Neoforge,
            // NeoForge 47 (Minecraft 1.20.1) still publishes itself as `forge`.
            ("net.neoforged", "forge") => {
                return Some((
                    ModLoaderType::Neoforge,
                    strip_minecraft_prefix(version, minecraft),
                ));
            }
            ("net.minecraftforge", "forge" | "fmlloader" | "minecraftforge") => {
                return Some((
                    ModLoaderType::Forge,
                    strip_minecraft_prefix(version, minecraft),
                ));
            }
            _ => continue,
        };
        return Some((kind, version.to_string()));
    }
    None
}

/// The strings of `arguments.game` plus the legacy `minecraftArguments`.
fn game_arguments(pack: &Pack) -> Vec<String> {
    let mut arguments: Vec<String> = pack
        .minecraft_arguments
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let Some(game) = pack.arguments.as_ref().map(|arguments| &arguments.game) else {
        return arguments;
    };
    for argument in game {
        match argument {
            Value::String(argument) => arguments.push(argument.clone()),
            Value::Object(object) => match object.get("value") {
                Some(Value::String(value)) => arguments.push(value.clone()),
                Some(Value::Array(values)) => arguments.extend(
                    values
                        .iter()
                        .filter_map(|value| value.as_str())
                        .map(str::to_string),
                ),
                _ => {}
            },
            _ => {}
        }
    }
    arguments
}

fn argument_value<'a>(arguments: &'a [String], flag: &str) -> Option<&'a str> {
    let index = arguments.iter().position(|argument| argument == flag)?;
    arguments.get(index + 1).map(String::as_str)
}

/// `group:artifact:version[:classifier]`, as a version manifest writes a library.
fn split_library(name: &str) -> Option<(&str, &str, &str)> {
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    (!group.is_empty() && !artifact.is_empty() && !version.is_empty())
        .then_some((group, artifact, version))
}

fn strip_minecraft_prefix(version: &str, minecraft: &str) -> String {
    let prefix = format!("{minecraft}-");
    version.strip_prefix(&prefix).unwrap_or(version).to_string()
}

fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8], name: &str) -> Result<T> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| Error::Manifest(format!("{name} is not UTF-8: {error}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text).map_err(|error| Error::Manifest(format!("invalid {name}: {error}")))
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

    fn pack(json: &str) -> Pack {
        parse(json.as_bytes(), VERSION_MANIFEST).expect("a valid version manifest")
    }

    #[test]
    fn reads_a_fabric_loader_from_the_libraries() {
        let pack = pack(
            r#"{
                "id": "1.20.1",
                "libraries": [
                    { "name": "net.fabricmc:fabric-loader:0.15.0" },
                    { "name": "net.fabricmc:intermediary:1.20.1" }
                ]
            }"#,
        );
        let (kind, version) = detect_loader(&pack, "1.20.1").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Fabric));
        assert_eq!(version, "0.15.0");
    }

    #[test]
    fn prefers_the_launch_argument_of_a_neo_forge_pack() {
        let pack = pack(
            r#"{
                "id": "1.20.4",
                "arguments": {
                    "game": [
                        "--fml.neoForgeVersion", "20.4.237",
                        "--fml.forgeVersion", "49.0.19",
                        { "value": "--launchTarget", "rules": [] }
                    ]
                }
            }"#,
        );
        let (kind, version) = detect_loader(&pack, "1.20.4").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Neoforge));
        assert_eq!(version, "20.4.237");
    }

    #[test]
    fn strips_a_minecraft_prefix_from_a_legacy_forge_library() {
        let pack = pack(
            r#"{
                "id": "1.12.2",
                "libraries": [{ "name": "net.minecraftforge:forge:1.12.2-14.23.5.2859" }]
            }"#,
        );
        let (kind, version) = detect_loader(&pack, "1.12.2").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Forge));
        assert_eq!(version, "14.23.5.2859");
    }

    #[test]
    fn reads_a_neoforge_47_library() {
        let pack = pack(
            r#"{
                "id": "1.20.1",
                "libraries": [{ "name": "net.neoforged:forge:1.20.1-47.1.106" }]
            }"#,
        );
        let (kind, version) = detect_loader(&pack, "1.20.1").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Neoforge));
        assert_eq!(version, "47.1.106");
    }

    #[test]
    fn reads_a_legacy_minecraft_arguments_line() {
        let pack = pack(
            r#"{
                "id": "1.7.10",
                "minecraftArguments": "--username ${auth_player_name} --fml.forgeVersion 10.13.4.1614"
            }"#,
        );
        let (kind, version) = detect_loader(&pack, "1.7.10").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Forge));
        assert_eq!(version, "10.13.4.1614");
    }

    #[test]
    fn a_vanilla_pack_has_no_loader() {
        let pack = pack(r#"{"id": "1.20.1", "libraries": []}"#);
        assert!(detect_loader(&pack, "1.20.1").is_none());
    }

    #[test]
    fn inspects_an_extracted_pack() {
        let directory =
            std::env::temp_dir().join(format!("conic-hmcl-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join("minecraft/mods")).expect("a temporary directory");
        std::fs::write(
            directory.join(MANIFEST),
            r#"{"name": "HMCL Pack", "version": "1.0", "gameVersion": "1.12.2"}"#,
        )
        .expect("a manifest");
        std::fs::write(
            directory.join(VERSION_MANIFEST),
            r#"{
                "id": "1.12.2-forge1.12.2-14.23.5.2859",
                "libraries": [{ "name": "net.minecraftforge:forge:1.12.2-14.23.5.2859" }]
            }"#,
        )
        .expect("a version manifest");
        std::fs::write(directory.join("minecraft/mods/a.jar"), "jar").expect("a mod");
        let inspection = crate::inspect(&directory).expect("an inspection");
        assert_eq!(inspection.name, "HMCL Pack");
        assert_eq!(inspection.minecraft, "1.12.2");
        assert!(matches!(inspection.loader_type, Some(ModLoaderType::Forge)));
        assert_eq!(inspection.loader_version.as_deref(), Some("14.23.5.2859"));
        let FormatInspection::Hmcl(inspection) = &inspection.format else {
            panic!("an HMCL pack");
        };
        assert_eq!(inspection.overrides.len(), 1);
        std::fs::remove_dir_all(&directory).ok();
    }
}
