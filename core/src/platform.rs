// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt::Display;

use os_info::{Type, Version};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OsFamily {
    Windows,
    Linux,
    Macos,
}

impl Display for OsFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Windows => write!(f, "windows"),
            Self::Linux => write!(f, "linux"),
            Self::Macos => write!(f, "macos"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlatformInfo {
    pub arch: String,
    pub arch_from_uname: Option<String>,
    pub os_type: Type,
    pub os_family: OsFamily,
    pub os_version: Version,
    pub edition: Option<String>,
}

#[cfg(windows)]
pub const DELIMITER: &str = ";";
#[cfg(not(windows))]
pub const DELIMITER: &str = ":";

impl PlatformInfo {
    /// get platform information
    pub fn new() -> Self {
        let os_family = if cfg!(target_os = "windows") {
            OsFamily::Windows
        } else if cfg!(target_os = "linux") {
            OsFamily::Linux
        } else if cfg!(target_os = "macos") {
            OsFamily::Macos
        } else {
            panic!("Sorry, but this program does not support your system!")
        };
        let os_info = os_info::get();
        Self {
            arch_from_uname: os_info.architecture().map(|x| x.to_owned()),
            os_family,
            os_version: os_info.version().to_owned(),
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
            os_type: os_info.os_type(),
            edition: os_info.edition().map(|x| x.to_owned()),
        }
    }
}
