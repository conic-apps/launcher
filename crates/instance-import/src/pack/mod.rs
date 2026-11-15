// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The modpack half of the crate: [`detect`] names the format of a dropped
//! pack and each module reads only its own manifest, handing back a [`Pack`]
//! that resolves into the common file list and trees.

pub(crate) mod curseforge;
pub(crate) mod hmcl;
pub(crate) mod mcbbs;
pub(crate) mod modrinth;
pub(crate) mod multimc;

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::error::{Error, Result};
use crate::source::Source;
use crate::{ImportKind, Metadata, Resolved};

/// The name a source falls back to when it names itself nothing: the dropped
/// path's own stem.
pub(crate) fn fallback_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Modpack".to_string())
}

/// A pack-supplied relative path, or `None` when it could leave the instance
/// root.
pub(crate) fn relative_path(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let path = Path::new(raw);
    let mut normal = 0;
    for component in path.components() {
        match component {
            Component::Normal(_) => normal += 1,
            _ => return None,
        }
    }
    (normal > 0).then(|| path.to_path_buf())
}

/// The format-specific half of an [`ImportSource`](crate::ImportSource).
#[derive(Clone)]
pub(crate) enum Pack {
    CurseForge(curseforge::Inspection),
    Modrinth(modrinth::Inspection),
    MultiMc(multimc::Inspection),
    Mcbbs(mcbbs::Inspection),
    Hmcl(hmcl::Inspection),
}

impl Pack {
    /// What the frontend words the import as.
    pub(crate) fn kind(&self) -> ImportKind {
        match self {
            // A Prism/MultiMC pack is an exported instance, and the same reader
            // takes a live instance folder, so both are an instance import.
            Self::MultiMc(_) => ImportKind::Instance,
            _ => ImportKind::Modpack,
        }
    }
}

#[derive(Debug)]
enum Format {
    CurseForge,
    Modrinth,
    MultiMc,
    Mcbbs,
    Hmcl,
}

const MCBBS_META: &str = "mcbbs.packmeta";
const MMC_MANIFEST: &str = "mmc-pack.json";
const MMC_CONFIG: &str = "instance.cfg";
const MODRINTH_MANIFEST: &str = "modrinth.index.json";
const CURSEFORGE_MANIFEST: &str = "manifest.json";
const HMCL_MANIFEST: &str = "modpack.json";

/// Every file name that identifies a pack. A dropped pack must expose one of
/// them at its root.
pub(crate) const MARKERS: &[&str] = &[
    MCBBS_META,
    MMC_MANIFEST,
    MMC_CONFIG,
    MODRINTH_MANIFEST,
    CURSEFORGE_MANIFEST,
    HMCL_MANIFEST,
];

pub(crate) fn inspect(path: &Path, source: &Source) -> Result<(Metadata, Pack)> {
    match detect(source)? {
        Format::CurseForge => curseforge::inspect(path, source)
            .map(|(metadata, inspection)| (metadata, Pack::CurseForge(inspection))),
        Format::Modrinth => modrinth::inspect(path, source)
            .map(|(metadata, inspection)| (metadata, Pack::Modrinth(inspection))),
        Format::MultiMc => multimc::inspect(path, source)
            .map(|(metadata, inspection)| (metadata, Pack::MultiMc(inspection))),
        Format::Mcbbs => mcbbs::inspect(path, source)
            .map(|(metadata, inspection)| (metadata, Pack::Mcbbs(inspection))),
        Format::Hmcl => hmcl::inspect(path, source)
            .map(|(metadata, inspection)| (metadata, Pack::Hmcl(inspection))),
    }
}

pub(crate) async fn resolve(pack: Pack) -> Result<Resolved> {
    match pack {
        Pack::CurseForge(inspection) => curseforge::resolve(inspection).await,
        Pack::Modrinth(inspection) => modrinth::resolve(inspection).await,
        Pack::MultiMc(inspection) => multimc::resolve(inspection),
        Pack::Mcbbs(inspection) => mcbbs::resolve(inspection).await,
        Pack::Hmcl(inspection) => hmcl::resolve(inspection),
    }
}

/// The order is the one a pack that carries more than one manifest is read in:
/// `manifest.json` is shared by CurseForge and MCBBS, and a pack exported from
/// another launcher can keep a nested `manifest.json` beside its own.
fn detect(source: &Source) -> Result<Format> {
    if source.has_file(MCBBS_META)? {
        return Ok(Format::Mcbbs);
    }
    if source.has_file(MMC_MANIFEST)? || source.has_file(MMC_CONFIG)? {
        return Ok(Format::MultiMc);
    }
    if source.has_file(MODRINTH_MANIFEST)? {
        return Ok(Format::Modrinth);
    }
    if source.has_file(CURSEFORGE_MANIFEST)? {
        // MCBBS reuses the name; its `addons` array is what tells them apart.
        let manifest = source.read(CURSEFORGE_MANIFEST)?;
        return Ok(if declares_addons(&manifest) {
            Format::Mcbbs
        } else {
            Format::CurseForge
        });
    }
    if source.has_file(HMCL_MANIFEST)? {
        return Ok(Format::Hmcl);
    }
    // `Source::try_open` only succeeds on a marker.
    Err(Error::Manifest(
        "the pack has no manifest this launcher recognises".to_string(),
    ))
}

/// A malformed `manifest.json` is left to the CurseForge reader, which reports
/// where it stopped parsing.
fn declares_addons(manifest: &[u8]) -> bool {
    serde_json::from_slice::<Value>(manifest)
        .ok()
        .and_then(|manifest| manifest.get("addons").cloned())
        .is_some_and(|addons| addons.is_array())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_file(name: &str, contents: &str) -> std::path::PathBuf {
        let directory =
            std::env::temp_dir().join(format!("conic-pack-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        std::fs::write(directory.join(name), contents).expect("a manifest");
        directory
    }

    #[test]
    fn detects_a_curseforge_directory() {
        let directory = pack_file(CURSEFORGE_MANIFEST, "{}");
        let source = Source::try_open(&directory, MARKERS)
            .expect("an opened path")
            .expect("a pack");
        assert!(matches!(detect(&source), Ok(Format::CurseForge)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn reads_an_mcbbs_manifest_that_reuses_the_curseforge_name() {
        let directory = pack_file(CURSEFORGE_MANIFEST, r#"{"addons": [], "files": []}"#);
        let source = Source::try_open(&directory, MARKERS)
            .expect("an opened path")
            .expect("a pack");
        assert!(matches!(detect(&source), Ok(Format::Mcbbs)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn prefers_the_multimc_manifest_over_a_nested_curseforge_one() {
        let directory = pack_file(MMC_MANIFEST, r#"{"components": []}"#);
        std::fs::write(directory.join(CURSEFORGE_MANIFEST), "{}").expect("a nested manifest");
        let source = Source::try_open(&directory, MARKERS)
            .expect("an opened path")
            .expect("a pack");
        assert!(matches!(detect(&source), Ok(Format::MultiMc)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_prism_instance_is_an_instance_import() {
        let directory = pack_file(MMC_CONFIG, "name=Pack\nInstanceType=OneSix\n");
        std::fs::write(
            directory.join(MMC_MANIFEST),
            r#"{"components": [{"uid": "net.minecraft", "version": "1.20.1"}]}"#,
        )
        .expect("a manifest");
        let import = crate::inspect(&directory).expect("an inspection");
        assert_eq!(import.kind(), ImportKind::Instance);
        std::fs::remove_dir_all(&directory).ok();
    }

    /// Writes a zip whose entries are `(name, contents)` pairs, and returns its
    /// path.
    fn pack_archive(entries: &[(&str, &str)]) -> std::path::PathBuf {
        use std::io::Write as _;

        let archive =
            std::env::temp_dir().join(format!("conic-pack-test-{}.zip", uuid::Uuid::new_v4()));
        let file = std::fs::File::create(&archive).expect("a zip file");
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, contents) in entries {
            zip.start_file(*name, options).expect("a zip entry");
            zip.write_all(contents.as_bytes()).expect("the entry");
        }
        zip.finish().expect("a finished zip");
        archive
    }

    #[tokio::test]
    async fn resolves_a_modrinth_archive_and_writes_both_override_trees() {
        let archive = pack_archive(&[
            (
                "pack/modrinth.index.json",
                r#"{
                    "formatVersion": 1,
                    "game": "minecraft",
                    "name": "Archived pack",
                    "dependencies": { "minecraft": "1.20.1", "fabric-loader": "0.15.0" },
                    "files": [{
                        "path": "mods/a.jar",
                        "hashes": { "sha1": "abc" },
                        "downloads": ["https://example.invalid/a.jar"],
                        "fileSize": 12
                    }]
                }"#,
            ),
            ("pack/overrides/config/a.toml", "value = 1\n"),
            ("pack/client-overrides/config/b.toml", "value = 2\n"),
        ]);
        let inspection = crate::inspect(&archive).expect("an inspection");
        assert_eq!(inspection.name, "Archived pack");
        assert_eq!(inspection.kind(), ImportKind::Modpack);
        let plan = crate::resolve(inspection).await.expect("a plan");
        assert!(plan.unavailable.is_empty());
        assert_eq!(plan.files.len(), 1);
        let root =
            std::env::temp_dir().join(format!("conic-pack-extract-{}", uuid::Uuid::new_v4()));
        crate::files::write_all(plan.trees, root.clone(), std::sync::Arc::new(|_| {}))
            .await
            .expect("the trees");
        assert!(root.join("config/a.toml").is_file());
        assert!(root.join("config/b.toml").is_file());
        std::fs::remove_file(&archive).ok();
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn resolves_a_prism_archive_into_the_game_directory_only() {
        let archive = pack_archive(&[
            (
                "My Pack/mmc-pack.json",
                r#"{
                    "formatVersion": 1,
                    "components": [
                        { "uid": "net.minecraft", "version": "1.20.1" },
                        { "uid": "net.fabricmc.fabric-loader", "version": "0.15.0" }
                    ]
                }"#,
            ),
            (
                "My Pack/instance.cfg",
                "name=Prism Pack\nInstanceType=OneSix\n",
            ),
            ("My Pack/.minecraft/mods/a.jar", "jar"),
            ("My Pack/.minecraft/logs/latest.log", "noise"),
            ("My Pack/patches/local.patch", "{}"),
        ]);
        let inspection = crate::inspect(&archive).expect("an inspection");
        assert_eq!(inspection.name, "Prism Pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        assert_eq!(inspection.kind(), ImportKind::Instance);
        let plan = crate::resolve(inspection).await.expect("a plan");
        assert!(plan.files.is_empty());
        let root =
            std::env::temp_dir().join(format!("conic-pack-extract-{}", uuid::Uuid::new_v4()));
        crate::files::write_all(plan.trees, root.clone(), std::sync::Arc::new(|_| {}))
            .await
            .expect("the trees");
        assert!(root.join("mods/a.jar").is_file());
        assert!(!root.join("logs/latest.log").exists());
        assert!(!root.join("mmc-pack.json").exists());
        assert!(!root.join("instance.cfg").exists());
        std::fs::remove_file(&archive).ok();
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn an_mcbbs_archive_only_fetches_the_files_it_does_not_ship() {
        let archive = pack_archive(&[
            (
                "mcbbs.packmeta",
                r#"{
                    "name": "MCBBS archive",
                    "addons": [{ "id": "game", "version": "1.12.2" }],
                    "fileApi": "https://example.invalid/api",
                    "files": [
                        { "type": "addon", "path": "mods/local.jar", "hash": "abc" },
                        { "type": "addon", "path": "mods/remote.jar", "hash": "def" }
                    ]
                }"#,
            ),
            ("overrides/mods/local.jar", "jar"),
        ]);
        let inspection = crate::inspect(&archive).expect("an inspection");
        assert_eq!(inspection.minecraft, "1.12.2");
        let crate::Recognized::Pack(Pack::Mcbbs(inspection)) = &inspection.recognized else {
            panic!("an MCBBS pack");
        };
        assert_eq!(inspection.addon_files.len(), 1);
        assert_eq!(inspection.addon_files[0].path, "mods/remote.jar");
        std::fs::remove_file(&archive).ok();
    }

    #[test]
    fn keeps_only_paths_inside_the_instance() {
        assert_eq!(
            crate::pack::relative_path("mods/a.jar"),
            Some(std::path::PathBuf::from("mods/a.jar"))
        );
        assert_eq!(crate::pack::relative_path("../mods/a.jar"), None);
        assert_eq!(crate::pack::relative_path("/mods/a.jar"), None);
        assert_eq!(crate::pack::relative_path("mods/../../a.jar"), None);
        assert_eq!(crate::pack::relative_path(""), None);
    }
}
