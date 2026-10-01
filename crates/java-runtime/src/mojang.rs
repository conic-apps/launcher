// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Path helpers for the Mojang-provided Java runtimes managed by the launcher.
//!
//! The launcher downloads these runtimes from Mojang into
//! `<data>/runtime/<platform_folder>/<component>` (see the `install` crate),
//! and both game launching and installer execution resolve their Java
//! executable through the helpers in this module.

use std::path::PathBuf;

use folder::DATA_LOCATION;
use platform::{OsArch, OsFamily, PLATFORM_INFO};

use crate::error::*;
use crate::models::JavaArch;

/// Maps the current OS architecture to the architecture of the Mojang-provided
/// Java runtime, which is always built for the host platform.
pub fn host_java_arch() -> JavaArch {
    match PLATFORM_INFO.arch {
        OsArch::X64 => JavaArch::X64,
        OsArch::X86 => JavaArch::X86,
        OsArch::Aarch64 => JavaArch::Aarch64,
        OsArch::Arm => JavaArch::Arm,
        _ => JavaArch::Unknown,
    }
}

/// Returns the installation directory of the launcher-managed runtime with the
/// given component name (e.g. `java-runtime-gamma`).
pub fn get_installation_directory(java_component: &str) -> Result<PathBuf> {
    let root = &DATA_LOCATION.runtime;
    let platform_folder_name = match PLATFORM_INFO.os_family {
        OsFamily::Windows => match PLATFORM_INFO.arch {
            OsArch::X64 => "windows_x64",
            OsArch::X86 => "windows_x86",
            OsArch::Aarch64 => "windows_arm64",
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
        OsFamily::Linux => match PLATFORM_INFO.arch {
            OsArch::X64 => "linux_amd64",
            OsArch::X86 => "linux_i386",
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
        OsFamily::Macos => match PLATFORM_INFO.arch {
            OsArch::X64 => "macos_x64",
            OsArch::Aarch64 => "macos_arm64",
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
    };
    Ok(root.join(platform_folder_name).join(java_component))
}

/// Returns the `java` executable path of the launcher-managed runtime with the
/// given component name.
pub fn get_executable_path(java_component: &str) -> Result<PathBuf> {
    let installation_directory = get_installation_directory(java_component)?;
    match PLATFORM_INFO.os_family {
        OsFamily::Linux => Ok(installation_directory.join("bin").join("java")),
        OsFamily::Macos => Ok(installation_directory
            .join("jre.bundle")
            .join("Contents")
            .join("Home")
            .join("bin")
            .join("java")),
        OsFamily::Windows => {
            if java_component == "minecraft-java-exe" {
                Ok(installation_directory.join("MinecraftJava.exe"))
            } else {
                Ok(installation_directory.join("bin").join("javaw.exe"))
            }
        }
    }
}
