// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game folders parser

use std::{
    ffi::OsStr,
    fmt::Display,
    format,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use log::error;
use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;

use platform::{OsFamily, PLATFORM_INFO};

pub static DATA_LOCATION: Lazy<DataLocation> = Lazy::new(DataLocation::default);

const DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("./launcher_profiles.json");

#[derive(Clone)]
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

    pub fn get_authlib_injector<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.get_version_root(version_id)
            .join("authlib-injector.jar")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DataLocation {
    pub root: PathBuf,
    pub instances: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub resources: PathBuf,
    pub temp: PathBuf,
    pub config: PathBuf,
}

impl DataLocation {
    pub fn new<S: AsRef<OsStr> + ?Sized>(data_folder: &S) -> Self {
        let data_folder_root = Path::new(data_folder).to_path_buf();
        let temp_path = std::env::temp_dir().join(format!(
            "conic-launcher-{}",
            uuid::Uuid::from_u128(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Incorrect System Time")
                    .as_nanos(),
            )
        ));
        std::fs::create_dir_all(&temp_path).expect("Could not create temp dir");
        Self {
            instances: data_folder_root.join("instances"),
            cache: match PLATFORM_INFO.os_family {
                OsFamily::Macos => data_folder_root.join(".cache"),
                OsFamily::Windows => data_folder_root.join(".cache"),
                OsFamily::Linux => {
                    PathBuf::from(std::env::var("HOME").expect("Could not found home"))
                        .join(".cache/conic")
                }
            },
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

    pub fn init(&self) {
        std::fs::create_dir_all(&self.root).expect("Unable to create application data directory");
        let launcher_profiles_path = self.root.join("launcher_profiles.json");
        let override_json_profile_result =
            std::fs::write(&launcher_profiles_path, DEFAULT_LAUNCHER_PROFILE);
        if override_json_profile_result.is_err() {
            error!("Unable to override launcher_profile.json, forge may not install properly")
        }
    }
}

impl Default for DataLocation {
    fn default() -> Self {
        #[cfg(not(debug_assertions))]
        #[allow(unused_variables)]
        let application_folder_name = "conic";
        #[cfg(debug_assertions)]
        #[allow(unused_variables)]
        let application_folder_name = "conic-debug";
        #[cfg(test)]
        let application_folder_name = "conic-test";
        let application_data_path = match PLATFORM_INFO.os_family {
            OsFamily::Windows => {
                PathBuf::from(std::env::var("APPDATA").expect("Could not found APP_DATA directory"))
                    .join(application_folder_name)
            }
            OsFamily::Macos => PathBuf::from("/Users/").join(application_folder_name),
            OsFamily::Linux => PathBuf::from(std::env::var("HOME").expect("Could not found home"))
                .join(format!(".{application_folder_name}")),
        };
        #[cfg(test)]
        {
            std::fs::remove_dir_all(&application_data_path).expect("Could not clear data folder");
            std::fs::create_dir_all(&application_data_path).expect("Could not create data folder");
        }
        Self::new(&application_data_path)
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("folder")
        .invoke_handler(tauri::generate_handler![cmd_get_data_location])
        .build()
}

#[command]
fn cmd_get_data_location() -> DataLocation {
    DATA_LOCATION.clone()
}
