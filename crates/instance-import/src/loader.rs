// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The loader a version manifest was built for, and the Minecraft version it
//! builds on.
//!
//! HMCL's `minecraft/pack.json` and the version JSON of a version-isolated
//! instance (the layout PCL and older HMCL use) are the same shape: a library
//! list plus, for old versions, a `minecraftArguments` line. Neither names its
//! loader in a field, so it is read off the library that provides it or the
//! `--fml.*Version` argument the loader adds.

use serde::Deserialize;
use serde_json::Value;

use instance::ModLoaderType;

use crate::error::{Error, Result};

#[derive(Deserialize, Default)]
pub(crate) struct VersionManifest {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "inheritsFrom")]
    inherits_from: Option<String>,
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

impl VersionManifest {
    pub(crate) fn parse(bytes: &[u8], name: &str) -> Result<Self> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| Error::Manifest(format!("{name} is not UTF-8: {error}")))?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        serde_json::from_str(text)
            .map_err(|error| Error::Manifest(format!("invalid {name}: {error}")))
    }

    /// The Minecraft version this manifest builds on.
    ///
    /// A version-isolated instance inherits from a plain version; a plain one
    /// carries its own id, which has to look like a version so a launcher's
    /// folder name is not mistaken for one.
    pub(crate) fn minecraft(&self) -> Option<&str> {
        if let Some(version) = self
            .inherits_from
            .as_deref()
            .map(str::trim)
            .filter(|version| !version.is_empty())
        {
            return Some(version);
        }
        let id = self.id.trim();
        looks_like_version(id).then_some(id)
    }

    /// The loader this manifest was built for.
    ///
    /// The launch arguments are checked first: they carry the loader version
    /// verbatim, while a library version can be prefixed with the Minecraft one.
    pub(crate) fn detect_loader(&self, minecraft: &str) -> Option<(ModLoaderType, String)> {
        let arguments = self.game_arguments();
        for (flag, kind) in [
            ("--fml.neoForgeVersion", ModLoaderType::Neoforge),
            ("--fml.forgeVersion", ModLoaderType::Forge),
        ] {
            if let Some(version) = argument_value(&arguments, flag) {
                return Some((kind, version.to_string()));
            }
        }
        for library in &self.libraries {
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
    fn game_arguments(&self) -> Vec<String> {
        let mut arguments: Vec<String> = self
            .minecraft_arguments
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let Some(game) = self.arguments.as_ref().map(|arguments| &arguments.game) else {
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
}

/// Forge ids carry the Minecraft version in front (`1.20.1-47.2.0`).
pub(crate) fn strip_minecraft_prefix(version: &str, minecraft: &str) -> String {
    let prefix = format!("{minecraft}-");
    version.strip_prefix(&prefix).unwrap_or(version).to_string()
}

/// Whether `id` names a Minecraft version rather than an instance folder.
///
/// A version is dots and digits, a snapshot (`24w14a`), or one of those with a
/// `-pre`/`-rc` suffix. An instance folder named after its version (`1.20.1-Forge`)
/// is deliberately not one: its own `inheritsFrom` is what says which version
/// it builds on.
fn looks_like_version(id: &str) -> bool {
    if snapshot(id) {
        return true;
    }
    let core = match id.split_once('-') {
        None => id,
        Some((core, suffix)) => {
            if !(suffix.starts_with("pre") || suffix.starts_with("rc")) {
                return false;
            }
            core
        }
    };
    !core.is_empty()
        && core
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
        && core.chars().any(|character| character.is_ascii_digit())
}

/// `24w14a`: two digits, `w`, two digits, a revision letter.
fn snapshot(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() >= 6
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b'w'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5].is_ascii_alphabetic()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> VersionManifest {
        VersionManifest::parse(json.as_bytes(), "pack.json").expect("a valid manifest")
    }

    #[test]
    fn reads_a_fabric_loader_from_the_libraries() {
        let manifest = manifest(
            r#"{
                "id": "1.20.1",
                "libraries": [
                    { "name": "net.fabricmc:fabric-loader:0.15.0" },
                    { "name": "net.fabricmc:intermediary:1.20.1" }
                ]
            }"#,
        );
        let (kind, version) = manifest.detect_loader("1.20.1").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Fabric));
        assert_eq!(version, "0.15.0");
    }

    #[test]
    fn prefers_the_launch_argument_of_a_neo_forge_pack() {
        let manifest = manifest(
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
        let (kind, version) = manifest.detect_loader("1.20.4").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Neoforge));
        assert_eq!(version, "20.4.237");
    }

    #[test]
    fn reads_a_neoforge_47_library() {
        let manifest = manifest(
            r#"{
                "id": "1.20.1",
                "libraries": [{ "name": "net.neoforged:forge:1.20.1-47.1.106" }]
            }"#,
        );
        let (kind, version) = manifest.detect_loader("1.20.1").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Neoforge));
        assert_eq!(version, "47.1.106");
    }

    #[test]
    fn strips_a_minecraft_prefix_from_a_legacy_forge_library() {
        let manifest = manifest(
            r#"{
                "id": "1.12.2",
                "libraries": [{ "name": "net.minecraftforge:forge:1.12.2-14.23.5.2859" }]
            }"#,
        );
        let (kind, version) = manifest.detect_loader("1.12.2").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Forge));
        assert_eq!(version, "14.23.5.2859");
    }

    #[test]
    fn reads_a_legacy_minecraft_arguments_line() {
        let manifest = manifest(
            r#"{
                "id": "1.7.10",
                "minecraftArguments": "--username ${auth_player_name} --fml.forgeVersion 10.13.4.1614"
            }"#,
        );
        let (kind, version) = manifest.detect_loader("1.7.10").expect("a loader");
        assert!(matches!(kind, ModLoaderType::Forge));
        assert_eq!(version, "10.13.4.1614");
    }

    #[test]
    fn a_vanilla_manifest_has_no_loader() {
        let manifest = manifest(r#"{"id": "1.20.1", "libraries": []}"#);
        assert!(manifest.detect_loader("1.20.1").is_none());
    }

    #[test]
    fn reads_the_version_a_manifest_builds_on() {
        let inherited = manifest(r#"{"id": "My Pack", "inheritsFrom": "1.20.1"}"#);
        assert_eq!(inherited.minecraft(), Some("1.20.1"));

        let plain = manifest(r#"{"id": "1.20.1"}"#);
        assert_eq!(plain.minecraft(), Some("1.20.1"));

        let snapshot = manifest(r#"{"id": "24w14a"}"#);
        assert_eq!(snapshot.minecraft(), Some("24w14a"));

        let folder = manifest(r#"{"id": "My Pack"}"#);
        assert_eq!(folder.minecraft(), None);
    }
}
