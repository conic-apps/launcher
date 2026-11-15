// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The pack-format layer: [`inspect`] detects the format from the dropped path
//! and asks it to read the pack; [`resolve`] dispatches the same way. The
//! installer only sees the common [`Plan`].

mod curseforge;
mod hmcl;
mod mcbbs;
mod modrinth;
mod multimc;
mod source;

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::error::{Error, Result};
use crate::overrides::Overrides;
use crate::{Inspection, Plan, ResolvedFile, UnavailableFile};
pub(crate) use source::Source;

/// The format-specific half of an [`Inspection`].
#[derive(Clone)]
pub(crate) enum FormatInspection {
    CurseForge(curseforge::Inspection),
    Modrinth(modrinth::Inspection),
    MultiMc(multimc::Inspection),
    Mcbbs(mcbbs::Inspection),
    Hmcl(hmcl::Inspection),
}

/// What a format resolves a pack into; the common [`Plan`] adds the pack
/// identity around it.
pub(crate) struct ResolvedPack {
    pub(crate) files: Vec<ResolvedFile>,
    pub(crate) unavailable: Vec<UnavailableFile>,
    pub(crate) overrides: Vec<Overrides>,
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
const MARKERS: &[&str] = &[
    MCBBS_META,
    MMC_MANIFEST,
    MMC_CONFIG,
    MODRINTH_MANIFEST,
    CURSEFORGE_MANIFEST,
    HMCL_MANIFEST,
];

pub(crate) fn inspect(path: &Path) -> Result<Inspection> {
    let source = Source::open(path, MARKERS)?;
    match detect(&source)? {
        Format::CurseForge => curseforge::inspect(path, &source),
        Format::Modrinth => modrinth::inspect(path, &source),
        Format::MultiMc => multimc::inspect(path, &source),
        Format::Mcbbs => mcbbs::inspect(path, &source),
        Format::Hmcl => hmcl::inspect(path, &source),
    }
}

pub(crate) async fn resolve(inspection: Inspection) -> Result<Plan> {
    let Inspection {
        name,
        minecraft,
        loader_type,
        loader_version,
        launch,
        format,
    } = inspection;
    let resolved = match format {
        FormatInspection::CurseForge(inspection) => curseforge::resolve(inspection).await?,
        FormatInspection::Modrinth(inspection) => modrinth::resolve(inspection).await?,
        FormatInspection::MultiMc(inspection) => multimc::resolve(inspection)?,
        FormatInspection::Mcbbs(inspection) => mcbbs::resolve(inspection).await?,
        FormatInspection::Hmcl(inspection) => hmcl::resolve(inspection)?,
    };
    Ok(Plan {
        name,
        minecraft,
        loader_type,
        loader_version,
        launch,
        unavailable: resolved.unavailable,
        files: resolved.files,
        overrides: resolved.overrides,
    })
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
    // `Source::open` only succeeds on a marker, so this is unreachable unless
    // a directory changed between the two reads.
    Err(Error::Manifest(format!(
        "{} does not contain a modpack manifest",
        source.describe()
    )))
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
            std::env::temp_dir().join(format!("conic-format-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        std::fs::write(directory.join(name), contents).expect("a manifest");
        directory
    }

    #[test]
    fn detects_a_curseforge_directory() {
        let directory = pack_file(CURSEFORGE_MANIFEST, "{}");
        let source = Source::open(&directory, MARKERS).expect("a pack");
        assert!(matches!(detect(&source), Ok(Format::CurseForge)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn reads_an_mcbbs_manifest_that_reuses_the_curseforge_name() {
        let directory = pack_file(CURSEFORGE_MANIFEST, r#"{"addons": [], "files": []}"#);
        let source = Source::open(&directory, MARKERS).expect("a pack");
        assert!(matches!(detect(&source), Ok(Format::Mcbbs)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn prefers_the_multimc_manifest_over_a_nested_curseforge_one() {
        let directory = pack_file(MMC_MANIFEST, r#"{"components": []}"#);
        std::fs::write(directory.join(CURSEFORGE_MANIFEST), "{}").expect("a nested manifest");
        let source = Source::open(&directory, MARKERS).expect("a pack");
        assert!(matches!(detect(&source), Ok(Format::MultiMc)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn rejects_a_folder_that_holds_no_manifest() {
        let directory =
            std::env::temp_dir().join(format!("conic-format-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        assert!(Source::open(&directory, MARKERS).is_err());
        std::fs::remove_dir_all(&directory).ok();
    }

    /// Writes a zip whose entries are `(name, contents)` pairs, and returns its
    /// path.
    fn pack_archive(entries: &[(&str, &str)]) -> std::path::PathBuf {
        use std::io::Write as _;

        let archive =
            std::env::temp_dir().join(format!("conic-format-test-{}.zip", uuid::Uuid::new_v4()));
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
    async fn resolves_a_modrinth_archive_and_extracts_both_override_trees() {
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
        let plan = crate::resolve(inspection).await.expect("a plan");
        assert!(plan.unavailable.is_empty());
        assert_eq!(plan.files.len(), 1);
        let root =
            std::env::temp_dir().join(format!("conic-format-extract-{}", uuid::Uuid::new_v4()));
        crate::overrides::extract_all(&plan.overrides, &root)
            .await
            .expect("the overrides");
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
            ("My Pack/patches/local.patch", "{}"),
        ]);
        let inspection = crate::inspect(&archive).expect("an inspection");
        assert_eq!(inspection.name, "Prism Pack");
        assert_eq!(inspection.minecraft, "1.20.1");
        let plan = crate::resolve(inspection).await.expect("a plan");
        assert!(plan.files.is_empty());
        let root =
            std::env::temp_dir().join(format!("conic-format-extract-{}", uuid::Uuid::new_v4()));
        crate::overrides::extract_all(&plan.overrides, &root)
            .await
            .expect("the overrides");
        assert!(root.join("mods/a.jar").is_file());
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
        let FormatInspection::Mcbbs(inspection) = &inspection.format else {
            panic!("an MCBBS pack");
        };
        assert_eq!(inspection.addon_files.len(), 1);
        assert_eq!(inspection.addon_files[0].path, "mods/remote.jar");
        std::fs::remove_file(&archive).ok();
    }

    #[test]
    fn keeps_only_paths_inside_the_instance() {
        assert_eq!(
            relative_path("mods/a.jar"),
            Some(PathBuf::from("mods/a.jar"))
        );
        assert_eq!(relative_path("../mods/a.jar"), None);
        assert_eq!(relative_path("/mods/a.jar"), None);
        assert_eq!(relative_path("mods/../../a.jar"), None);
        assert_eq!(relative_path(""), None);
    }
}
