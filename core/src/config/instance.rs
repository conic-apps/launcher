// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use serde::{Deserialize, Serialize};

use super::launch::{Server, GC};

#[derive(Deserialize, Serialize)]
pub enum ModLoaderType {
    Fabric,
    Forge,
    Quilt,
    Neoforged,
}

impl fmt::Display for ModLoaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fabric => {
                write!(f, "Fabric")
            }
            Self::Quilt => {
                write!(f, "Quilt")
            }
            Self::Forge => {
                write!(f, "Forge")
            }
            Self::Neoforged => {
                write!(f, "Neoforged")
            }
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct InstanceRuntime {
    pub minecraft: String,
    pub mod_loader_type: Option<ModLoaderType>,
    pub mod_loader_version: Option<String>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct InstanceLaunchConfig {
    pub enable_instance_specific_settings: bool,
    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: Option<usize>,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: Option<usize>,
    pub(crate) server: Option<Server>,
    /// window width
    pub(crate) width: Option<usize>,

    /// window height
    pub(crate) height: Option<usize>,

    pub(crate) fullscreen: Option<bool>,

    /// User custom additional java virtual machine command line arguments.
    pub(crate) extra_jvm_args: Option<String>,

    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: Option<String>,

    pub(crate) is_demo: Option<bool>,

    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: Option<bool>,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: Option<bool>,

    /// Add extra classpath
    pub(crate) extra_class_paths: Option<String>,

    pub(crate) gc: Option<GC>,

    pub(crate) launcher_name: Option<String>,
    pub wrap_command: Option<String>,

    pub execute_before_launch: Option<String>,

    pub execute_after_launch: Option<String>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct InstanceConfig {
    pub name: String,
    pub runtime: InstanceRuntime,
    #[serde(default)]
    pub group: Option<Vec<String>>,
    #[serde(default)]
    pub launch_config: InstanceLaunchConfig,
}

impl InstanceConfig {
    pub fn new(instance_name: &str, minecraft_version: &str) -> Self {
        Self {
            name: instance_name.to_string(),
            runtime: InstanceRuntime {
                minecraft: minecraft_version.to_string(),
                mod_loader_type: None,
                mod_loader_version: None,
            },
            group: None,
            launch_config: InstanceLaunchConfig::default(),
        }
    }
}
