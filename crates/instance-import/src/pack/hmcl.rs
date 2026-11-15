// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The HMCL pack format: an old-style HMCL export with a `modpack.json`
//! describing the pack, a `minecraft/pack.json` version manifest, and the game
//! files under `minecraft/`.
//!
//! Everything the pack needs is in the archive, so nothing is downloaded. The
//! loader is not named anywhere directly; `pack.json` carries it as a library
//! or an `--fml.*Version` launch argument, which is what [`VersionManifest`]
//! reads.

use std::path::Path;

use serde::Deserialize;

use crate::Metadata;
use crate::error::{Error, Result};
use crate::files::FileTree;
use crate::loader::VersionManifest;
use crate::settings::Settings;
use crate::source::Source;

const MANIFEST: &str = "modpack.json";
/// The pack's game directory, and the version manifest HMCL writes in it.
const GAME_DIRECTORY: &str = "minecraft";
const VERSION_MANIFEST: &str = "minecraft/pack.json";
/// Content of the game directory that this launcher installs itself.
const OWN_CONTENT: &[&str] = &["pack.json", "versions", "libraries", "assets"];

#[derive(Clone)]
pub(crate) struct Inspection {
    trees: Vec<FileTree>,
}

#[derive(Deserialize)]
struct Modpack {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "gameVersion")]
    game_version: String,
}

/// Reads an HMCL pack into the common metadata and its own half.
pub(super) fn inspect(path: &Path, source: &Source) -> Result<(Metadata, Inspection)> {
    let modpack: Modpack = parse(&source.read(MANIFEST)?, MANIFEST)?;
    let manifest = if source.has_file(VERSION_MANIFEST)? {
        Some(VersionManifest::parse(
            &source.read(VERSION_MANIFEST)?,
            VERSION_MANIFEST,
        )?)
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
    let (loader_type, loader_version) = match manifest
        .as_ref()
        .and_then(|manifest| manifest.detect_loader(&minecraft))
    {
        Some((kind, version)) => (Some(kind), Some(version)),
        None => {
            if manifest.is_some() {
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
        super::fallback_name(path)
    } else {
        name.to_string()
    };
    log::info!("import '{name}': HMCL, Minecraft {minecraft}");
    let trees = if source.has_tree(GAME_DIRECTORY)? {
        vec![source.tree(GAME_DIRECTORY).skipping(OWN_CONTENT)]
    } else {
        Vec::new()
    };
    Ok((
        Metadata {
            name,
            minecraft,
            loader_type,
            loader_version,
            settings: Settings::default(),
        },
        Inspection { trees },
    ))
}

/// Resolves an HMCL pack into the common plan; it downloads nothing.
pub(super) fn resolve(inspection: Inspection) -> Result<crate::Resolved> {
    Ok(crate::Resolved {
        files: Vec::new(),
        unavailable: Vec::new(),
        trees: inspection.trees,
    })
}

fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8], name: &str) -> Result<T> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| Error::Manifest(format!("{name} is not UTF-8: {error}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text).map_err(|error| Error::Manifest(format!("invalid {name}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(
            inspection.loader_type,
            Some(instance::ModLoaderType::Forge)
        ));
        assert_eq!(inspection.loader_version.as_deref(), Some("14.23.5.2859"));
        let crate::Recognized::Pack(crate::pack::Pack::Hmcl(inspection)) = &inspection.recognized
        else {
            panic!("an HMCL pack");
        };
        assert_eq!(inspection.trees.len(), 1);
        std::fs::remove_dir_all(&directory).ok();
    }
}
