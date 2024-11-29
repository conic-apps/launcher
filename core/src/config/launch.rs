// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

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

impl Default for GC {
    fn default() -> Self {
        Self::G1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct LaunchConfig {
    #[serde(default)]
    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: usize,

    #[serde(default = "default_max_memory")]
    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: usize,
    #[serde(default)]
    pub(crate) server: Option<Server>,
    #[serde(default = "default_width")]
    /// window width
    pub(crate) width: usize,

    #[serde(default = "default_height")]
    /// window height
    pub(crate) height: usize,

    #[serde(default)]
    pub(crate) fullscreen: bool,

    #[serde(default)]
    /// User custom additional java virtual machine command line arguments.
    pub(crate) extra_jvm_args: String,

    #[serde(default)]
    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: String,

    #[serde(default)]
    pub(crate) is_demo: bool,

    #[serde(default)]
    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: bool,

    #[serde(default)]
    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: bool,

    #[serde(default)]
    /// Add extra classpath
    pub(crate) extra_class_paths: String,

    #[serde(default)]
    /// Choose Java Garbage Collection
    pub(crate) gc: GC,

    #[serde(default = "default_launcher_name")]
    pub(crate) launcher_name: String,

    #[serde(default)]
    /// Add this to the front of launch command
    pub wrap_command: String,

    #[serde(default)]
    pub execute_before_launch: String,

    #[serde(default)]
    pub execute_after_launch: String,

    #[serde(default)]
    /// Skip refresh account before launch
    pub skip_refresh_account: bool,

    #[serde(default)]
    /// Skip check game file integrity
    pub skip_check_files: bool,
}

fn default_max_memory() -> usize {
    2048
}
fn default_width() -> usize {
    854
}
fn default_height() -> usize {
    480
}

fn default_launcher_name() -> String {
    "Amethyst_Launcher".to_string()
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            min_memory: 0,
            max_memory: default_max_memory(),
            server: None,
            width: default_width(),
            height: default_height(),
            fullscreen: false,
            extra_jvm_args: String::new(),
            extra_mc_args: String::new(),
            is_demo: false,
            ignore_invalid_minecraft_certificates: false,
            ignore_patch_discrepancies: false,
            extra_class_paths: String::new(),
            gc: GC::default(),
            launcher_name: default_launcher_name(),
            wrap_command: String::new(),
            execute_after_launch: String::new(),
            execute_before_launch: String::new(),
            skip_refresh_account: false,
            skip_check_files: false,
        }
    }
}
