// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt::Display;

use once_cell::sync::Lazy;
use os_info::{Type, Version};
use serde::{Deserialize, Serialize};
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};

pub static PLATFORM_INFO: Lazy<PlatformInfo> = Lazy::new(PlatformInfo::new);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("platform")
        .invoke_handler(tauri::generate_handler![cmd_get_platform_info])
        .build()
}

#[command]
fn cmd_get_platform_info() -> PlatformInfo {
    PLATFORM_INFO.clone()
}
/// Represents the high-level operating system family.
///
/// This is an abstraction over detailed OS types (e.g., Ubuntu, Windows 10) to group
/// them by family: Windows, Linux, or macOS.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OsFamily {
    /// Microsoft Windows OS family
    Windows,

    /// Linux-based distributions (e.g., Ubuntu, Arch, Debian)
    Linux,

    /// Apple macOS family
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

/// Contains detailed platform-related information, such as architecture,
/// OS type, version, and edition.
///
/// Typically used for environment-specific behavior or diagnostics.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlatformInfo {
    /// The target CPU architecture (e.g., "x64", "arm").
    pub arch: String,

    /// The architecture string as reported by `uname`, if available.
    pub arch_from_uname: Option<String>,

    /// The operating system type, as reported by the `os_info` crate.
    pub os_type: Type,

    /// The general OS family classification (Windows/Linux/macOS).
    pub os_family: OsFamily,

    /// The version of the OS (e.g., 10.15.7, 22.04, etc.).
    pub os_version: Version,

    /// The edition of the OS (e.g., "Home", "Professional"), if available.
    pub edition: Option<String>,
}

/// The path delimiter character used in environment variables like `PATH`.
///
/// On Windows, this is `";"`, and on other systems it is `":"`.
#[cfg(windows)]
pub const DELIMITER: &str = ";";
#[cfg(not(windows))]
pub const DELIMITER: &str = ":";

impl PlatformInfo {
    /// Constructs a new [`PlatformInfo`] instance using compile-time and runtime system data.
    ///
    /// - Detects architecture using `cfg!(target_arch)`
    /// - Detects OS family using `cfg!(target_os)`
    /// - Uses `os_info` crate to get detailed version, type, and edition info
    ///
    /// # Panics
    /// Panics if the OS is not supported by the program.
    #[allow(clippy::new_without_default)]
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
