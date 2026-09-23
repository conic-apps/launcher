// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use once_cell::sync::Lazy;
use os_info::{Type, Version};
use serde::{Deserialize, Serialize};

/// Tauri-free mirror of `crates/platform`. The original crate is a Tauri
/// plugin; this one keeps the same data model without the Tauri dependency
/// so the Slint app can interrogate the OS directly.
pub static PLATFORM_INFO: Lazy<PlatformInfo> = Lazy::new(PlatformInfo::new);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OsArch {
    X64,
    X86,
    Mips,
    PowerPC,
    PowerPC64,
    Arm,
    Aarch64,
    Unknown,
}

/// Represents the high-level operating system family.
///
/// This is an abstraction over detailed OS types (e.g., Ubuntu, Windows 10)
/// to group them by family: Windows, Linux, or macOS.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OsFamily {
    /// Microsoft Windows OS family
    Windows,

    /// Linux-based distributions (e.g., Ubuntu, Arch, Debian)
    Linux,

    /// Apple macOS family
    Macos,
}

/// Contains detailed platform-related information, such as architecture,
/// OS type, version, and edition.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlatformInfo {
    /// The real hardware CPU architecture, detected at runtime via `os_info`.
    pub arch: OsArch,

    /// The operating system type, as reported by the `os_info` crate.
    pub os_type: Type,

    /// The general OS family classification (Windows/Linux/macOS).
    pub os_family: OsFamily,

    /// The version of the OS (e.g., 10.15.7, 22.04, etc.).
    pub os_version: Version,

    /// The edition of the OS (e.g., "Home", "Professional"), if available.
    pub edition: Option<String>,
}

impl PlatformInfo {
    /// Constructs a new [`PlatformInfo`] instance using runtime system data.
    ///
    /// - Detects hardware architecture at runtime via `os_info`
    /// - Detects OS family using `cfg!(target_os)`
    /// - Uses the `os_info` crate to get detailed version, type, and edition info
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
            arch: parse_arch(os_info.architecture()),
            os_family,
            os_version: os_info.version().to_owned(),
            os_type: os_info.os_type(),
            edition: os_info.edition().map(|x| x.to_owned()),
        }
    }
}

impl std::fmt::Display for OsFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsFamily::Windows => f.write_str("windows"),
            OsFamily::Linux => f.write_str("linux"),
            OsFamily::Macos => f.write_str("macos"),
        }
    }
}

fn parse_arch(arch_str: Option<&str>) -> OsArch {
    match arch_str {
        Some("x86_64") => OsArch::X64,
        Some("amd64") => OsArch::X64,
        Some("i386") => OsArch::X86,
        Some("mips") => OsArch::Mips,
        Some("powerpc") => OsArch::PowerPC,
        Some("powerpc64") => OsArch::PowerPC64,
        Some("arm") => OsArch::Arm,
        Some("armv7l") => OsArch::Arm,
        Some("armv7") => OsArch::Arm,
        Some("aarch64") => OsArch::Aarch64,
        Some("arm64") => OsArch::Aarch64,
        _ => OsArch::Unknown,
    }
}
