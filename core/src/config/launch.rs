// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// TODO: launch options

// now just use default launch config

use serde::{Deserialize, Serialize};

/// Game process priority, invalid on windows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum ProcessPriority {
    High,
    AboveNormal,
    Normal,
    BelowNormal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Server {
    pub ip: String,
    pub port: Option<u16>,
}

/// User custom jvm gc
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum GC {
    Serial,
    Parallel,
    ParallelOld,
    G1,
    Z,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct LaunchConfig {
    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: u32,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: u32,
    pub(crate) server: Option<Server>,
    /// window width
    pub(crate) width: u32,

    /// window height
    pub(crate) height: u32,

    pub(crate) fullscreen: bool,

    /// User custom additional java virtual machine command line arguments.
    pub(crate) extra_jvm_args: String,

    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: String,

    pub(crate) is_demo: bool,
    /// Game process priority, invalid on windows
    pub(crate) process_priority: ProcessPriority,

    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: bool,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: bool,

    /// Add extra classpath
    pub(crate) extra_class_paths: String,

    pub(crate) gc: GC,

    pub(crate) launcher_name: String,
    pub wrap_command: String,

    pub execute_before_launch: String,

    pub execute_after_launch: String,
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
            process_priority: ProcessPriority::Normal,
            ignore_invalid_minecraft_certificates: false,
            ignore_patch_discrepancies: false,
            extra_class_paths: String::new(),
            gc: GC::G1,
            launcher_name: "Amethyst_Launcher".to_string(),
            wrap_command: String::new(),
            execute_after_launch: String::new(),
            execute_before_launch: String::new(),
        }
    }
}
