// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Selection of the Java executable used to launch the game or run mod loader
//! installers.
//!
//! The resolution order mirrors what players expect from a launcher:
//!
//! 1. An instance-specific `java_path`, when configured by the user;
//! 2. When `prefer_mojang_java` is enabled, the Mojang-provided runtime
//!    installed under the launcher runtime directory;
//! 3. A system-installed Java runtime matching the required major version,
//!    excluding launcher-managed and user-disabled runtimes.

use std::path::PathBuf;

use log::info;

use crate::error::*;
use crate::models::JavaArch;
use crate::mojang;
use crate::scan_java_runtimes;

/// A resolved Java runtime: the executable path together with its architecture.
#[derive(Debug, Clone)]
pub struct ResolvedJava {
    /// Absolute path of the `java` executable.
    pub path: PathBuf,
    pub arch: JavaArch,
}

/// Inputs for [`resolve_java_executable`].
///
/// Plain values are taken instead of the launcher `Config`/`Instance` types so
/// this crate stays independent of them.
#[derive(Debug, Clone, Default)]
pub struct ResolveJavaOptions {
    /// Instance-specific `java` executable configured by the user.
    pub instance_java_path: Option<String>,
    /// Whether the Mojang-provided runtime is preferred over system runtimes.
    pub prefer_mojang_java: bool,
    /// User-disabled Java executable paths.
    pub disabled_java_runtimes: Vec<String>,
    /// Required Java major version of the game version.
    pub required_major_version: u32,
    /// Mojang runtime component of the game version (e.g. `java-runtime-gamma`).
    pub mojang_component: String,
}

/// Resolves the Java executable used to run the game or a mod loader installer.
///
/// The returned [`ResolvedJava::arch`] carries the bitness of the runtime:
/// Mojang-provided Java always matches the OS architecture, while scanned
/// system runtimes report their own parsed architecture. The instance-specific
/// path is reported as [`JavaArch::Unknown`] since it is not parsed again.
///
/// Returns [`Error::NoSuitableJavaRuntime`] when no usable runtime is found.
pub async fn resolve_java_executable(options: &ResolveJavaOptions) -> Result<ResolvedJava> {
    if let Some(java_path) = &options.instance_java_path {
        info!("Using instance-specific Java: {java_path}");
        return Ok(ResolvedJava {
            path: PathBuf::from(java_path),
            arch: JavaArch::Unknown,
        });
    }

    if options.prefer_mojang_java
        && let Ok(mojang_path) = mojang::get_executable_path(&options.mojang_component)
    {
        if mojang_path.is_file() {
            info!("Using Mojang-provided Java: {}", mojang_path.display());
            return Ok(ResolvedJava {
                path: mojang_path,
                arch: mojang::host_java_arch(),
            });
        }
        info!(
            "Mojang-provided Java not found at {}, falling back to system Java",
            mojang_path.display()
        );
    }

    let required_major_version = options.required_major_version;
    let disabled_java_runtimes = &options.disabled_java_runtimes;
    let runtimes = tauri::async_runtime::spawn_blocking(scan_java_runtimes).await??;
    let system_java = runtimes.into_iter().find(|runtime| {
        runtime.major_version == required_major_version
            && runtime.is_valid
            && !runtime.is_managed
            && !disabled_java_runtimes
                .iter()
                .any(|disabled| runtime.path == std::path::Path::new(disabled))
    });
    if let Some(runtime) = system_java {
        info!("Using system Java: {}", runtime.path.display());
        return Ok(ResolvedJava {
            path: runtime.path,
            arch: runtime.arch,
        });
    }
    Err(Error::NoSuitableJavaRuntime)
}
