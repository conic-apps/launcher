/*
 * Amethyst Launcher Core
 * Copyright (C) 2023 Broken-Deer <old_driver__@outlook.com> and contributors
 *
 * This program is free software, you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! The core of the entire crate.
//!
//! # Example
//!
//! Get platform information:
//!
//! ```rust
//! async fn fn_name() {
//!     use aml_core::core::PlatformInfo;
//!     let platform_info = PlatformInfo::new().await;
//! }
//! ```
//!
//! Parse version.json:
//!
//! ```
//! use aml_core::core::folder::MinecraftLocation;
//! use aml_core::core::PlatformInfo;
//! use aml_core::core::version::Version;
//!
//!  async fn fn_name() {
//!     let version = reqwest::get("https://piston-meta.mojang.com/v1/packages/715ccf3330885e75b205124f09f8712542cbe7e0/1.20.1.json")
//!         .await
//!         .unwrap()
//!         .json::<Version>()
//!         .await
//!         .unwrap();
//!     let resolved = version.parse(&MinecraftLocation::new(".minecraft"), &PlatformInfo::new().await).await.unwrap();
//! }
//! ```
//!
//! Parse Minecraft folders:
//!
//! ```
//! use std::path::Path;
//! use aml_core::core::folder::MinecraftLocation;
//!
//! let minecraft_location = MinecraftLocation::new(".minecraft");
//! assert_eq!(minecraft_location.mods, Path::new(".minecraft/mods").to_path_buf());
//! ```
//!

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri_plugin_http::reqwest;
use tokio::process::Command;

pub mod folder;
// pub mod task;
pub mod version;

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);
/// May not actually be used
pub static DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("./launcher_profile.json");

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OsType {
    Windows,
    Linux,
    Osx,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlatformInfo {
    pub arch: String,
    pub name: String,
    pub os_type: OsType,
    pub version: String,
}

#[cfg(windows)]
pub static DELIMITER: &str = ";";
#[cfg(not(windows))]
pub static DELIMITER: &str = ":";

impl PlatformInfo {
    /// get platform information
    pub async fn new() -> Self {
        let os_type = if cfg!(target_os = "windows") {
            OsType::Windows
        } else if cfg!(target_os = "linux") {
            OsType::Linux
        } else if cfg!(target_os = "macos") {
            OsType::Osx
        } else {
            panic!("Sorry, but this program does not support your system!")
        };
        Self {
            name: match os_type {
                OsType::Windows => "windows".to_string(),
                OsType::Linux => "linux".to_string(),
                OsType::Osx => "osx".to_string(),
            },
            os_type,
            version: {
                #[cfg(windows)]
                {
                    use regex::Regex;

                    let mut command = Command::new("C:\\Windows\\System32\\cmd.exe");
                    command.creation_flags(0x08000000);
                    command.args(&[
                        "/C",
                        r#"powershell -c [System.Environment]::OSVersion.Version"#,
                    ]);
                    let output = command.output().await.unwrap();
                    let stdout = String::from_utf8(output.stdout).unwrap();

                    let regex = Regex::new(r"\s+").unwrap();
                    regex.replace_all(&stdout, ".").to_string()
                }
                #[cfg(not(windows))]
                {
                    let mut command = Command::new("uname");
                    command.args(["-r"]);
                    let output = command.output().await.unwrap();
                    String::from_utf8(output.stdout).unwrap()
                }
            },
            arch: if cfg!(target_arch = "x86_64") {
                "x64"
            } else if cfg!(target_arch = "x86") {
                "x86"
            } else if cfg!(target_arch = "mips") {
                "mips"
            } else if cfg!(target_arch = "powerpc") {
                "powerpc"
            } else if cfg!(target_arch = "powerpc64") {
                "powerpc64"
            } else if cfg!(target_arch = "arm") {
                "arm"
            } else if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                "unknown"
            }
            .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JavaExec {
    pub binary: PathBuf,
    // pub version: String,
    // pub version_major: String,
}

impl JavaExec {
    pub async fn new<P: AsRef<OsStr> + ?Sized>(home: &P) -> Self {
        let home = Path::new(home).to_path_buf();
        // let release = tokio::fs::read_to_string(home.join("release"))
        //     .await
        //     .unwrap();
        // let version = release
        //     .lines()
        //     .find(|line| line.starts_with("JAVA_VERSION"))
        //     .unwrap()
        //     .split("=")
        //     .collect::<Vec<&str>>()
        //     .get(1)
        //     .unwrap()
        //     .trim()
        //     .to_string();
        Self {
            binary: home.join("bin").join("java"),
            // version_major: version.split(".").collect::<Vec<&str>>().get(0).unwrap().to_string(),
            // version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Download {
    pub url: String,
    pub file: PathBuf,
    pub sha1: Option<String>,
}
