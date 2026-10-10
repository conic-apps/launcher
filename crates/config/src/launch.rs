// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// A server the game enters automatically at launch.
#[derive(Clone, Serialize, Deserialize)]
pub struct Server {
    pub ip: String,
    /// Optional port number of the server, default is 25565.
    pub port: Option<u16>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub enum GC {
    Serial,
    Parallel,
    #[default]
    G1,
    Z,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchConfig {
    /// Whether to automatically allocate memory for the game based on the
    /// available physical memory, following the PCL algorithm.
    pub auto_memory: bool,

    /// Maximum memory to allocate (MB), passed as `-Xmx`.
    ///
    /// Only used when [`LaunchConfig::auto_memory`] is disabled.
    pub max_memory: usize,

    pub server: Option<Server>,

    pub width: usize,

    pub height: usize,

    pub fullscreen: bool,

    pub extra_jvm_args: String,

    pub extra_mc_args: String,

    pub is_demo: bool,

    /// Adds `-Dfml.ignoreInvalidMinecraftCertificates=true` to JVM args.
    pub ignore_invalid_minecraft_certificates: bool,

    /// Adds `-Dfml.ignorePatchDiscrepancies=true` to JVM args.
    pub ignore_patch_discrepancies: bool,

    pub extra_class_paths: String,

    pub gc: GC,

    pub launcher_name: String,

    pub wrap_command: String,

    pub execute_before_launch: String,

    pub execute_after_launch: String,

    pub skip_refresh_account: bool,

    pub skip_check_files: bool,

    pub quit_app_after_launch: bool,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            auto_memory: true,
            max_memory: 2048,
            server: None,
            width: 854,
            height: 480,
            fullscreen: false,
            extra_jvm_args: String::new(),
            extra_mc_args: String::new(),
            is_demo: false,
            ignore_invalid_minecraft_certificates: false,
            ignore_patch_discrepancies: false,
            extra_class_paths: String::new(),
            gc: GC::default(),
            launcher_name: "Conic_Launcher".to_string(),
            wrap_command: String::new(),
            execute_after_launch: String::new(),
            execute_before_launch: String::new(),
            skip_refresh_account: false,
            skip_check_files: false,
            quit_app_after_launch: false,
        }
    }
}
