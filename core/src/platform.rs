// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use tokio::process::Command;

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
pub const DELIMITER: &str = ";";
#[cfg(not(windows))]
pub const DELIMITER: &str = ":";

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
                    command.args([
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
