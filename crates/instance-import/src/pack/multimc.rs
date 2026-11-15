// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Prism/MultiMC instance format: `mmc-pack.json` names the game and loader
//! components, `instance.cfg` holds the instance's own launch settings, and the
//! game files sit under `.minecraft/`.
//!
//! A pack can also carry `patches/` and `jarmods/`, which patch the game jar
//! itself; neither is reproduced here, so the pack is installed without them and
//! the log says so.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use instance::ModLoaderType;

use crate::Metadata;
use crate::error::{Error, Result};
use crate::files::FileTree;
use crate::loader::strip_minecraft_prefix;
use crate::settings::Settings;
use crate::source::Source;

const MANIFEST: &str = "mmc-pack.json";
const CONFIG: &str = "instance.cfg";
/// Where the instance's own game directory is inside the pack.
const GAME_DIRECTORY: &str = ".minecraft";

const GAME: &str = "net.minecraft";
const FORGE: &str = "net.minecraftforge";
const NEOFORGE: &str = "net.neoforged";
const FABRIC: &str = "net.fabricmc.fabric-loader";
const QUILT: &str = "org.quiltmc.quilt-loader";

/// The Prism/MultiMC half of an inspection: everything here is already in the
/// pack, so only the game directory is left for the import.
#[derive(Clone)]
pub(crate) struct Inspection {
    trees: Vec<FileTree>,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    components: Vec<Component>,
}

#[derive(Deserialize)]
struct Component {
    uid: String,
    #[serde(default)]
    version: Option<String>,
    /// A component only another one needs; it is installed with its dependant.
    #[serde(default, rename = "dependencyOnly")]
    dependency_only: bool,
    #[serde(default)]
    important: bool,
}

/// Reads a Prism/MultiMC instance into the common metadata and its own half.
pub(super) fn inspect(path: &Path, source: &Source) -> Result<(Metadata, Inspection)> {
    let names = source.names()?;
    if !names.iter().any(|name| name == MANIFEST) {
        return Err(Error::Manifest(
            "the Prism/MultiMC instance has no mmc-pack.json".to_string(),
        ));
    }
    // `instance.cfg` is what MultiMC itself keys an instance off; a pack that
    // lost it is still readable through the component list.
    let config = names
        .iter()
        .any(|name| name == CONFIG)
        .then(|| source.read(CONFIG))
        .transpose()?
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .map(|text| parse_ini(&text))
        .unwrap_or_default();
    let manifest = parse_manifest(&source.read(MANIFEST)?)?;
    let (minecraft, loader_type, loader_version) = read_components(&manifest)?;
    let name = config
        .get("name")
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| super::fallback_name(path));
    log::info!(
        "import '{name}': Prism/MultiMC, Minecraft {minecraft}, {} components",
        manifest.components.len()
    );
    // A live instance carries its logs and crashes; they are not game files.
    let trees = if source.has_tree(GAME_DIRECTORY)? {
        vec![
            source
                .tree(GAME_DIRECTORY)
                .skipping(&["logs", "crash-reports"]),
        ]
    } else {
        Vec::new()
    };
    for unsupported in ["patches", "jarmods"] {
        if names
            .iter()
            .any(|name| name.starts_with(&format!("{unsupported}/")))
        {
            log::warn!(
                "the pack patches the game jar through {unsupported}/, which this launcher does \
                 not reproduce; those files are skipped"
            );
        }
    }
    Ok((
        Metadata {
            name,
            minecraft,
            loader_type,
            loader_version,
            settings: read_launch(&config),
        },
        Inspection { trees },
    ))
}

/// Resolves a Prism/MultiMC instance into the common plan; it downloads nothing.
pub(super) fn resolve(inspection: Inspection) -> Result<crate::Resolved> {
    Ok(crate::Resolved {
        files: Vec::new(),
        unavailable: Vec::new(),
        trees: inspection.trees,
    })
}

fn read_components(manifest: &Manifest) -> Result<(String, Option<ModLoaderType>, Option<String>)> {
    let mut minecraft = None;
    let mut loader_type = None;
    let mut loader_version = None;
    for component in &manifest.components {
        let version = component.version.as_deref().unwrap_or_default();
        match component.uid.as_str() {
            GAME => minecraft = Some(version.to_string()),
            FORGE | NEOFORGE => {
                loader_type = Some(if component.uid == FORGE {
                    ModLoaderType::Forge
                } else {
                    ModLoaderType::Neoforge
                });
                loader_version = Some(version.to_string());
            }
            FABRIC => {
                loader_type = Some(ModLoaderType::Fabric);
                loader_version = Some(version.to_string());
            }
            QUILT => {
                loader_type = Some(ModLoaderType::Quilt);
                loader_version = Some(version.to_string());
            }
            other => {
                if !component.dependency_only {
                    log::warn!(
                        "skipping the component '{other}'{}: this launcher does not install it",
                        if component.important {
                            " (the pack marks it important)"
                        } else {
                            ""
                        }
                    );
                }
                continue;
            }
        }
    }
    let minecraft = minecraft
        .filter(|minecraft| !minecraft.trim().is_empty())
        .ok_or_else(|| {
            Error::Manifest("the Prism/MultiMC manifest names no Minecraft component".to_string())
        })?;
    if loader_type.is_some()
        && loader_version
            .as_deref()
            .is_none_or(|version| version.trim().is_empty())
    {
        return Err(Error::Loader(
            "the Prism/MultiMC manifest names a mod loader without a version".to_string(),
        ));
    }
    // Forge and NeoForge ids are written with the Minecraft version in front
    // (`1.20.1-47.2.0`), the same way the CurseForge manifest does.
    let loader_version = loader_version
        .filter(|version| !version.trim().is_empty())
        .map(|version| match loader_type {
            Some(ModLoaderType::Forge | ModLoaderType::Neoforge) => {
                strip_minecraft_prefix(&version, &minecraft)
            }
            _ => version,
        });
    Ok((minecraft, loader_type, loader_version))
}

/// The settings `instance.cfg` asks a launcher to apply to this instance.
///
/// `JavaPath` is deliberately ignored: it is the author's own installation, and
/// pointing a launch at it would break on every machine but theirs.
fn read_launch(config: &HashMap<String, String>) -> Settings {
    let mut launch = Settings::default();
    let mut jvm_arguments = Vec::new();
    if flag(config, "OverrideJavaArgs")
        && let Some(arguments) = config.get("JvmArgs")
    {
        jvm_arguments.push(arguments.clone());
    }
    if flag(config, "OverrideMemory") {
        // The launcher has no minimum-heap setting of its own, so the pack's
        // `MinMemAlloc` travels with the other JVM arguments.
        if let Some(minimum) = positive(config, "MinMemAlloc")
            && !jvm_arguments
                .iter()
                .any(|argument| argument.contains("-Xms"))
        {
            jvm_arguments.push(format!("-Xms{minimum}M"));
        }
        if let Some(maximum) = positive(config, "MaxMemAlloc") {
            launch.set_max_memory(maximum);
        }
    }
    launch.set_extra_jvm_args(jvm_arguments);
    if flag(config, "JoinServerOnLaunch")
        && let Some(address) = config.get("JoinServerOnLaunchAddress")
    {
        launch.set_server(address);
    }
    if flag(config, "OverrideWindow") {
        let width = positive(config, "MinecraftWinWidth").unwrap_or_default();
        let height = positive(config, "MinecraftWinHeight").unwrap_or_default();
        launch.set_window(width, height);
    }
    launch
}

fn positive(config: &HashMap<String, String>, key: &str) -> Option<u64> {
    config
        .get(key)
        .and_then(|value| value.trim().parse().ok())
        .filter(|value| *value > 0)
}

fn flag(config: &HashMap<String, String>, key: &str) -> bool {
    config
        .get(key)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

/// Reads the `key=value` lines of `instance.cfg`.
///
/// The file is Qt's INI dialect: keys without a group are written before any
/// heading (or under `[General]`), values escape `\`, newline, tab and `;`, and
/// every other heading is a group this instance does not use.
fn parse_ini(text: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let mut in_other_group = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(heading) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            in_other_group = !heading.trim().eq_ignore_ascii_case("General");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if in_other_group {
            continue;
        }
        values.insert(key.trim().to_string(), unescape(value.trim()));
    }
    values
}

fn unescape(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            unescaped.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => unescaped.push('\n'),
            Some('t') => unescaped.push('\t'),
            Some('r') => unescaped.push('\r'),
            Some(escape) => unescaped.push(escape),
            None => unescaped.push('\\'),
        }
    }
    unescaped
}

fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| Error::Manifest(format!("mmc-pack.json is not UTF-8: {error}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text)
        .map_err(|error| Error::Manifest(format!("invalid mmc-pack.json: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> Manifest {
        parse_manifest(json.as_bytes()).expect("a valid manifest")
    }

    #[test]
    fn reads_the_game_and_loader_from_the_components() {
        let manifest = manifest(
            r#"{
                "formatVersion": 1,
                "components": [
                    { "uid": "net.minecraft", "version": "1.20.1", "important": true },
                    { "uid": "net.minecraftforge", "version": "1.20.1-47.2.0", "important": true },
                    { "uid": "org.lwjgl3", "version": "3.3.1", "dependencyOnly": true }
                ]
            }"#,
        );
        let (minecraft, loader, version) = read_components(&manifest).expect("the components");
        assert_eq!(minecraft, "1.20.1");
        assert!(matches!(loader, Some(ModLoaderType::Forge)));
        assert_eq!(version.as_deref(), Some("47.2.0"));
    }

    #[test]
    fn reads_a_fabric_component() {
        let manifest = manifest(
            r#"{
                "components": [
                    { "uid": "net.minecraft", "version": "1.21.1" },
                    { "uid": "net.fabricmc.fabric-loader", "version": "0.16.0" },
                    { "uid": "net.fabricmc.intermediary", "version": "1.21.1", "dependencyOnly": true }
                ]
            }"#,
        );
        let (_, loader, version) = read_components(&manifest).expect("the components");
        assert!(matches!(loader, Some(ModLoaderType::Fabric)));
        assert_eq!(version.as_deref(), Some("0.16.0"));
    }

    #[test]
    fn requires_a_minecraft_component() {
        let manifest = manifest(r#"{"components": []}"#);
        assert!(read_components(&manifest).is_err());
    }

    #[test]
    fn reads_the_launch_settings_of_an_instance() {
        let config = parse_ini(
            "[General]\n\
             InstanceType=OneSix\n\
             name=My Pack\n\
             OverrideJavaArgs=true\n\
             JvmArgs=-XX:+UseG1GC -Dsun.stdout.encoding=UTF-8\n\
             OverrideMemory=true\n\
             MaxMemAlloc=6144\n\
             JoinServerOnLaunch=true\n\
             JoinServerOnLaunchAddress=play.example.com:25566\n\
             \n\
             [Controls]\n\
             key.reloadResourcePacks=KEY_F3\n",
        );
        assert_eq!(config.get("name").map(String::as_str), Some("My Pack"));
        let launch = read_launch(&config);
        let mut applied = instance::InstanceConfig::default();
        launch.apply(&mut applied);
        assert_eq!(
            applied.launch_config.extra_jvm_args.as_deref(),
            Some("-XX:+UseG1GC -Dsun.stdout.encoding=UTF-8")
        );
        assert_eq!(applied.launch_config.max_memory, Some(6144));
        let server = applied.launch_config.server.expect("a server");
        assert_eq!(server.ip, "play.example.com");
        assert_eq!(server.port, Some(25566));
    }

    #[test]
    fn carries_the_minimum_heap_as_a_jvm_argument() {
        let config = parse_ini(
            "OverrideJavaArgs=true\nJvmArgs=-XX:+UseG1GC\nOverrideMemory=true\nMinMemAlloc=1024\nMaxMemAlloc=6144\n",
        );
        let mut applied = instance::InstanceConfig::default();
        read_launch(&config).apply(&mut applied);
        assert_eq!(
            applied.launch_config.extra_jvm_args.as_deref(),
            Some("-XX:+UseG1GC -Xms1024M")
        );
        assert_eq!(applied.launch_config.max_memory, Some(6144));
    }

    #[test]
    fn ignores_an_override_the_instance_did_not_enable() {
        let config = parse_ini("name=Pack\nMaxMemAlloc=6144\nJvmArgs=-Xmx8G\n");
        assert!(read_launch(&config).is_empty());
    }

    #[test]
    fn unescapes_a_qt_ini_value() {
        assert_eq!(unescape(r#"-Da\\b -Dc\nd"#), "-Da\\b -Dc\nd");
    }

    #[test]
    fn inspects_an_extracted_instance() {
        let directory =
            std::env::temp_dir().join(format!("conic-multimc-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join(".minecraft/mods")).expect("a temporary directory");
        std::fs::write(
            directory.join(MANIFEST),
            r#"{
                "formatVersion": 1,
                "components": [
                    { "uid": "net.minecraft", "version": "1.20.1" },
                    { "uid": "org.quiltmc.quilt-loader", "version": "0.26.0" }
                ]
            }"#,
        )
        .expect("a manifest");
        std::fs::write(
            directory.join(CONFIG),
            "name=Quilt Pack\nInstanceType=OneSix\n",
        )
        .expect("an instance config");
        std::fs::write(directory.join(".minecraft/mods/a.jar"), "jar").expect("a mod");
        let inspection = crate::inspect(&directory).expect("an inspection");
        assert_eq!(inspection.name, "Quilt Pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        assert!(matches!(inspection.loader_type, Some(ModLoaderType::Quilt)));
        assert_eq!(inspection.loader_version.as_deref(), Some("0.26.0"));
        let crate::Recognized::Pack(crate::pack::Pack::MultiMc(inspection)) =
            &inspection.recognized
        else {
            panic!("a Prism/MultiMC pack");
        };
        assert_eq!(inspection.trees.len(), 1);
        std::fs::remove_dir_all(&directory).ok();
    }
}
