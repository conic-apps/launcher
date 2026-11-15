// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A version-isolated instance folder: the layout PCL keeps under
//! `versions/<name>`, and what an older HMCL export unpacks to.
//!
//! The folder holds a `<name>.json` version manifest next to the game files.
//! The manifest is the only thing that says which game and loader the instance
//! runs, so a folder without one is not this layout.

use std::path::Path;

use crate::error::Result;
use crate::files::FileTree;
use crate::loader::VersionManifest;
use crate::settings::Settings;
use crate::{Metadata, Resolved};

/// What of such a folder is the launcher's own, not a game file.
const LAUNCHER_CONTENT: &[&str] = &[
    "versions",
    "libraries",
    "assets",
    "logs",
    "crash-reports",
    "PCL",
    ".hmcl.json",
];

#[derive(Clone)]
pub(crate) struct Inspection {
    tree: FileTree,
}

pub(crate) fn inspect(path: &Path) -> Result<Option<(Metadata, Inspection)>> {
    if !path.is_dir() {
        return Ok(None);
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(None);
    };
    let manifest_path = path.join(format!("{name}.json"));
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let Ok(bytes) = std::fs::read(&manifest_path) else {
        return Ok(None);
    };
    let manifest_name = manifest_path.display().to_string();
    let Ok(manifest) = VersionManifest::parse(&bytes, &manifest_name) else {
        return Ok(None);
    };
    let Some(minecraft) = manifest.minecraft() else {
        return Ok(None);
    };
    let (loader_type, loader_version) = match manifest.detect_loader(minecraft) {
        Some((kind, version)) => (Some(kind), Some(version)),
        None => (None, None),
    };
    log::info!("import '{name}': version-isolated instance, Minecraft {minecraft}");
    let mut skip: Vec<String> = LAUNCHER_CONTENT
        .iter()
        .map(|name| name.to_string())
        .collect();
    // The manifest and the jar are the launcher's copy of the game; the installer
    // fetches its own.
    skip.push(format!("{name}.json"));
    skip.push(format!("{name}.jar"));
    let tree = FileTree::directory(path.to_path_buf(), String::new()).skipping_owned(skip);
    Ok(Some((
        Metadata {
            name: name.to_string(),
            minecraft: minecraft.to_string(),
            loader_type,
            loader_version,
            settings: Settings::default(),
        },
        Inspection { tree },
    )))
}

impl Inspection {
    pub(crate) fn resolve(self) -> Resolved {
        Resolved {
            files: Vec::new(),
            unavailable: Vec::new(),
            trees: vec![self.tree],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImportKind;

    fn instance_folder(manifest: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("My Pack-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join("mods")).expect("a temporary directory");
        let name = directory
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a name");
        std::fs::write(directory.join(format!("{name}.json")), manifest).expect("a manifest");
        std::fs::write(directory.join("mods/a.jar"), "jar").expect("a mod");
        directory
    }

    #[test]
    fn reads_a_version_isolated_instance() {
        let directory = instance_folder(
            r#"{
                "id": "My Pack",
                "inheritsFrom": "1.20.1",
                "libraries": [{ "name": "net.fabricmc:fabric-loader:0.15.0" }]
            }"#,
        );
        let import = crate::inspect(&directory).expect("an inspection");
        assert_eq!(import.kind(), ImportKind::Instance);
        assert_eq!(import.minecraft, "1.20.1");
        assert!(matches!(
            import.loader_type,
            Some(instance::ModLoaderType::Fabric)
        ));
        assert_eq!(import.loader_version.as_deref(), Some("0.15.0"));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[tokio::test]
    async fn copies_the_game_files_and_leaves_the_launcher_content() {
        let directory = instance_folder(
            r#"{
                "id": "My Pack",
                "inheritsFrom": "1.20.1",
                "libraries": []
            }"#,
        );
        std::fs::create_dir_all(directory.join("logs")).expect("a logs folder");
        std::fs::create_dir_all(directory.join("libraries")).expect("a libraries folder");
        std::fs::write(directory.join("logs/latest.log"), "noise").expect("a log");
        std::fs::write(directory.join("libraries/ignored.jar"), "jar").expect("a library");
        let name = directory
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a name")
            .to_string();
        std::fs::write(directory.join(format!("{name}.jar")), "jar").expect("the game jar");

        let (_, inspection) = inspect(&directory)
            .expect("an inspection")
            .expect("the layout");
        let root =
            std::env::temp_dir().join(format!("conic-version-copy-{}", uuid::Uuid::new_v4()));
        crate::files::write_all(
            inspection.resolve().trees,
            root.clone(),
            std::sync::Arc::new(|_| {}),
        )
        .await
        .expect("the copy");

        assert!(root.join("mods/a.jar").is_file());
        assert!(!root.join("logs").exists());
        assert!(!root.join("libraries").exists());
        assert!(!root.join(format!("{name}.json")).exists());
        assert!(!root.join(format!("{name}.jar")).exists());
        std::fs::remove_dir_all(&directory).ok();
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_folder_without_a_version_manifest_is_not_an_instance() {
        let directory =
            std::env::temp_dir().join(format!("conic-version-dir-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        assert!(inspect(&directory).expect("an inspection").is_none());
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_json_that_is_not_a_version_manifest_is_not_an_instance() {
        let directory = instance_folder(r#"{"name": "something else", "id": 600}"#);
        assert!(inspect(&directory).expect("an inspection").is_none());
        std::fs::remove_dir_all(&directory).ok();
    }
}
