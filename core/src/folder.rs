// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game folders parser
//!
//! # Example
//!
//! ```
//! use std::path::Path;
//! use aml_core::core::folder::MinecraftLocation;
//!
//! let minecraft_location = MinecraftLocation::new(".minecraft");
//!
//! assert_eq!(Path::new(".minecraft/mods").to_path_buf(), minecraft_location.mods);
//! assert_eq!(
//!     Path::new(".minecraft/versions/1.19.4/1.19.4.json").to_path_buf(),
//!     minecraft_location.get_version_json("1.19.4")
//! );
//! ```

use std::{
    ffi::OsStr,
    fmt::Display,
    format,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{platform::OsType, PLATFORM_INFO};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The Minecraft folder structure. All method will return the path related to a minecraft root like .minecraft.
pub struct MinecraftLocation {
    pub root: PathBuf,
    pub libraries: PathBuf,
    pub assets: PathBuf,
    pub versions: PathBuf,
}

impl MinecraftLocation {
    pub fn new<S: AsRef<OsStr> + ?Sized>(root: &S) -> MinecraftLocation {
        let root = Path::new(root);
        MinecraftLocation {
            root: root.to_path_buf(),
            assets: root.join("assets"),
            libraries: root.join("libraries"),
            versions: root.join("versions"),
        }
    }

    pub fn get_natives_root<P: AsRef<Path>>(&self, version: P) -> PathBuf {
        self.get_version_root(version).join("conic-natives")
    }

    pub fn get_version_root<P: AsRef<Path>>(&self, version: P) -> PathBuf {
        self.versions.join(version)
    }

    pub fn get_version_json<P: AsRef<Path> + Display>(&self, version: P) -> PathBuf {
        self.get_version_root(&version)
            .join(format!("{version}.json"))
    }

    pub fn get_version_jar<P: AsRef<Path> + Display>(
        &self,
        version: P,
        r#type: Option<&str>,
    ) -> PathBuf {
        if r#type == Some("client") || r#type.is_none() {
            self.get_version_root(&version)
                .join(format!("{version}.jar"))
        } else {
            self.get_version_root(&version)
                .join(format!("{version}-{}.jar", r#type.unwrap()))
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

    pub fn get_log_config<P: AsRef<Path>>(&self, version: P) -> PathBuf {
        self.get_version_root(version).join("log4j2.xml")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLocation {
    pub root: PathBuf,
    pub instances: PathBuf,
    pub cache: PathBuf,
    pub default_jre: PathBuf,
    pub logs: PathBuf,
    pub resources: PathBuf,
    pub temp: PathBuf,
    pub config: PathBuf,
}

impl DataLocation {
    pub fn new<S: AsRef<OsStr> + ?Sized>(data_folder: &S) -> Self {
        let data_folder_root = Path::new(data_folder).to_path_buf();
        let temp_path = std::env::temp_dir().join(format!("conic-launcher-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_path).expect("Could not create temp dir");
        Self {
            instances: data_folder_root.join("instances"),
            cache: match PLATFORM_INFO.get().unwrap().os_type {
                OsType::Linux => {
                    PathBuf::from(std::env::var("HOME").expect("Could not found home"))
                        .join(".cache/conic")
                }
                OsType::Osx => data_folder_root.join(".cache"),
                OsType::Windows => data_folder_root.join(".cache"),
            },
            // default_jre: data_folder.join("default_jre").join("bin").join("java"),
            default_jre: PathBuf::from_str("/bin/java").unwrap(),
            resources: data_folder_root.join("resources"),
            logs: data_folder_root.join("logs"),
            temp: temp_path,
            config: data_folder_root.join("config.toml"),
            root: data_folder_root,
        }
    }

    pub fn get_instance_root(&self, instance_id: &Uuid) -> PathBuf {
        self.instances.join(instance_id.to_string())
    }
}
