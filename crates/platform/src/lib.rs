// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use once_cell::sync::Lazy;
use os_info::{Type, Version};
use serde::{Deserialize, Serialize};

mod memory;

pub use memory::get_available_memory_bytes;

/// The host platform, detected once on first use.
pub static PLATFORM_INFO: Lazy<PlatformInfo> = Lazy::new(PlatformInfo::new);

/// The path delimiter character used in environment variables like `PATH`.
#[cfg(windows)]
pub const DELIMITER: &str = ";";
#[cfg(not(windows))]
pub const DELIMITER: &str = ":";

/// Strips the Windows `\\?\` UNC prefix added by [`std::fs::canonicalize`].
///
/// Some programs (e.g. Java) do not understand extended-length paths,
/// so this helper reverts the prefix while keeping the resolved absolute path.
#[cfg(windows)]
pub fn strip_unc_prefix(path: std::path::PathBuf) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(stripped) => std::path::PathBuf::from(stripped),
        None => path,
    }
}

#[cfg(not(windows))]
pub fn strip_unc_prefix(path: std::path::PathBuf) -> std::path::PathBuf {
    path
}

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
/// Groups the detailed `os_info` types (Ubuntu, Windows 10, …) into the three
/// families the rest of the app branches on.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OsFamily {
    Windows,

    Linux,

    Macos,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlatformInfo {
    /// The real hardware CPU architecture, detected at runtime via `os_info`.
    pub arch: OsArch,

    pub os_type: Type,

    pub os_family: OsFamily,

    pub os_version: Version,

    pub edition: Option<String>,
}

impl PlatformInfo {
    /// Constructs a new [`PlatformInfo`] instance using runtime system data.
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
        let info = Self {
            arch: parse_arch(os_info.architecture()),
            os_family,
            os_version: os_info.version().to_owned(),
            os_type: os_info.os_type(),
            edition: os_info.edition().map(|x| x.to_owned()),
        };
        // The one line that answers "what did the launcher think it was running
        // on". The app logs the family separately, but not the version, edition
        // or arch — and `main` writes no log before this is first touched.
        log::info!(
            "Platform: {} {} ({:?}, edition {:?}, arch {:?})",
            info.os_type,
            info.os_version,
            info.os_family,
            info.edition,
            info.arch
        );
        info
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
    // An unrecognised architecture silently becomes `Unknown`, and that changes
    // three separate decisions: the memory heuristics, the `arch` segment of the
    // update artifact name, and which native libraries a game gets. So the raw
    // string is recorded — nothing else would explain the outcome.
    if let Some(raw) = arch_str
        && !matches!(
            raw,
            "x86_64"
                | "amd64"
                | "i386"
                | "mips"
                | "powerpc"
                | "powerpc64"
                | "arm"
                | "armv7l"
                | "aarch64"
                | "arm64"
                | "riscv64"
                | "s390x"
                | "loongarch64"
        )
    {
        log::warn!("the architecture was reported as {raw:?}, which is not recognised");
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_architecture_names_map_to_their_variant() {
        assert_eq!(parse_arch(Some("x86_64")), OsArch::X64);
        assert_eq!(parse_arch(Some("amd64")), OsArch::X64);
        assert_eq!(parse_arch(Some("i386")), OsArch::X86);
        assert_eq!(parse_arch(Some("aarch64")), OsArch::Aarch64);
        assert_eq!(parse_arch(Some("arm64")), OsArch::Aarch64);
        assert_eq!(parse_arch(Some("armv7l")), OsArch::Arm);
    }

    #[test]
    fn an_unknown_or_missing_name_is_unknown() {
        assert_eq!(parse_arch(Some("riscv64")), OsArch::Unknown);
        assert_eq!(parse_arch(None), OsArch::Unknown);
    }

    #[test]
    fn the_os_family_display_matches_the_version_json_vocabulary() {
        assert_eq!(OsFamily::Windows.to_string(), "windows");
        assert_eq!(OsFamily::Linux.to_string(), "linux");
        assert_eq!(OsFamily::Macos.to_string(), "macos");
    }
}
