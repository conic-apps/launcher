// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The shared Minecraft installation.

use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

/// The Minecraft folder structure. Every method returns a path under a
/// Minecraft root such as `.minecraft`.
///
/// One of these is shared by every instance — it is where `versions`,
/// `libraries` and `assets` live — and it is not the launcher's data root: the
/// two are chosen separately (see [`crate::LauncherLocation`]).
#[derive(Debug, Clone)]
pub struct MinecraftLocation {
    pub root: PathBuf,
    pub libraries: PathBuf,
    pub assets: PathBuf,
    pub versions: PathBuf,
    /// The Mojang Java runtimes the launcher installs.
    pub runtime: PathBuf,
}

impl MinecraftLocation {
    pub fn new(root: impl AsRef<Path>) -> MinecraftLocation {
        let root = root.as_ref().to_path_buf();
        MinecraftLocation {
            assets: root.join("assets"),
            libraries: root.join("libraries"),
            versions: root.join("versions"),
            runtime: root.join("runtime"),
            root,
        }
    }

    pub fn get_natives_root<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.get_version_root(version_id).join("conic-natives")
    }

    pub fn get_version_root<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.versions.join(version_id)
    }

    pub fn get_version_json<P: AsRef<Path> + Display>(&self, version_id: P) -> PathBuf {
        self.get_version_root(&version_id)
            .join(format!("{version_id}.json"))
    }

    pub fn get_version_jar<P: AsRef<Path> + Display>(
        &self,
        version: P,
        version_jar_type: Option<&str>,
    ) -> PathBuf {
        if let Some(version_jar_type) = version_jar_type
            && version_jar_type != "client"
        {
            self.get_version_root(&version)
                .join(format!("{version}-{}.jar", version_jar_type))
        } else {
            self.get_version_root(&version)
                .join(format!("{version}.jar"))
        }
    }

    pub fn get_library_by_path<P: AsRef<Path>>(&self, library_path: P) -> PathBuf {
        self.libraries.join(library_path)
    }

    pub fn get_assets_index(&self, version_assets: &str) -> PathBuf {
        self.assets
            .join("indexes")
            .join(format!("{version_assets}.json"))
    }

    pub fn get_log_config<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.get_version_root(version_id).join("log4j2.xml")
    }

    /// Creates the root and writes the `launcher_profiles.json` the Forge and
    /// NeoForge installers read to find the installation.
    pub fn init(&self) {
        if let Err(error) = std::fs::create_dir_all(&self.root) {
            log::warn!(
                "Could not create the Minecraft directory {}: {error}",
                self.root.display()
            );
        }
        let launcher_profiles_path = self.root.join("launcher_profiles.json");
        if let Err(error) = std::fs::write(&launcher_profiles_path, DEFAULT_LAUNCHER_PROFILE) {
            log::error!(
                "Unable to write {}; forge may not install properly: {error}",
                launcher_profiles_path.display()
            );
        }
    }
}

const DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("./launcher_profiles.json");
