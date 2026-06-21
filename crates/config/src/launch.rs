// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// Represents a Minecraft server configuration, the game will enter the
/// server automatically.
#[derive(Clone, Serialize, Deserialize)]
pub struct Server {
    /// The IP address or hostname of the server.
    pub ip: String,
    /// Optional port number of the server, default is 25565.
    pub port: Option<u16>,
}

/// Enum representing the Java Garbage Collection algorithms.
#[derive(Clone, Serialize, Deserialize, Default)]
pub enum GC {
    /// Serial GC.
    Serial,
    /// Parallel GC (young generation).
    Parallel,
    /// Parallel GC (old generation).
    ParallelOld,
    /// G1 Garbage Collector (default).
    #[default]
    G1,
    /// Z Garbage Collector.
    Z,
}

/// Represents a configuration object for launching a Minecraft instance.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchConfig {
    /// Minimum memory to allocate (MB), passed as `-Xms`.
    pub min_memory: usize,

    /// Maximum memory to allocate (MB), passed as `-Xmx`.
    pub max_memory: usize,

    /// Optional server to connect to when the game launches.
    pub server: Option<Server>,

    /// Width of the game window in pixels.
    pub width: usize,

    /// Height of the game window in pixels.
    pub height: usize,

    /// Whether to launch the game in fullscreen mode.
    pub fullscreen: bool,

    /// Additional JVM arguments specified by the user.
    pub extra_jvm_args: String,

    /// Additional Minecraft command-line arguments specified by the user.
    pub extra_mc_args: String,

    /// Whether to launch in demo mode.
    pub is_demo: bool,

    /// Adds `-Dfml.ignoreInvalidMinecraftCertificates=true` to JVM args.
    pub ignore_invalid_minecraft_certificates: bool,

    /// Adds `-Dfml.ignorePatchDiscrepancies=true` to JVM args.
    pub ignore_patch_discrepancies: bool,

    /// Additional classpath entries to include.
    pub extra_class_paths: String,

    /// Selected Java Garbage Collector.
    pub gc: GC,

    /// The name of the launcher, passed to the game.
    pub launcher_name: String,

    /// A command prefix to wrap around the launch command.
    pub wrap_command: String,

    /// Script or command to execute before launching the game.
    pub execute_before_launch: String,

    /// Script or command to execute after the game exits.
    pub execute_after_launch: String,

    /// If true, skips refreshing the account before launch.
    pub skip_refresh_account: bool,

    /// If true, skips integrity checks of the game files.
    pub skip_check_files: bool,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            min_memory: 0,
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
        }
    }
}
