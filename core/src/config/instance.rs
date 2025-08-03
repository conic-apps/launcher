// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use serde::{Deserialize, Serialize};

use super::launch::{Server, GC};

/// Represents supported mod loader types.
#[derive(Deserialize, Serialize)]
pub enum ModLoaderType {
    /// Fabric mod loader
    Fabric,
    /// Quilt mod loader
    Quilt,
    /// Forge mod loader
    Forge,
    /// Neoforged mod loader
    Neoforged,
}

impl fmt::Display for ModLoaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fabric => write!(f, "Fabric"),
            Self::Quilt => write!(f, "Quilt"),
            Self::Forge => write!(f, "Forge"),
            Self::Neoforged => write!(f, "Neoforged"),
        }
    }
}

/// Defines the runtime environment for a Minecraft instance.
#[derive(Deserialize, Serialize, Default)]
pub struct InstanceRuntime {
    /// Minecraft version (e.g., "1.20.1")
    pub minecraft: String,

    /// Optional mod loader type (e.g., Forge, Fabric)
    pub mod_loader_type: Option<ModLoaderType>,

    /// Optional mod loader version (e.g., "44.1.23")
    pub mod_loader_version: Option<String>,
}

/// Configuration for how the instance should be launched.
#[derive(Deserialize, Serialize, Default)]
pub struct InstanceLaunchConfig {
    /// Whether to use instance-specific settings
    pub enable_instance_specific_settings: bool,

    /// Minimum allocated memory in MB (adds `-Xms` to JVM args)
    pub(crate) min_memory: Option<usize>,

    /// Maximum allocated memory in MB (adds `-Xmx` to JVM args)
    pub(crate) max_memory: Option<usize>,

    /// Minecraft server configuration for the instance
    pub(crate) server: Option<Server>,

    /// Game window width in pixels
    pub(crate) width: Option<usize>,

    /// Game window height in pixels
    pub(crate) height: Option<usize>,

    /// Whether to launch in fullscreen mode
    pub(crate) fullscreen: Option<bool>,

    /// Additional JVM arguments specified by user
    pub(crate) extra_jvm_args: Option<String>,

    /// Additional Minecraft arguments specified by user
    pub(crate) extra_mc_args: Option<String>,

    /// Whether to launch the game in demo mode
    pub(crate) is_demo: Option<bool>,

    /// Adds `-Dfml.ignoreInvalidMinecraftCertificates=true` to JVM args
    pub(crate) ignore_invalid_minecraft_certificates: Option<bool>,

    /// Adds `-Dfml.ignorePatchDiscrepancies=true` to JVM args
    pub(crate) ignore_patch_discrepancies: Option<bool>,

    /// Extra class paths to include in launch
    pub(crate) extra_class_paths: Option<String>,

    /// Garbage collection configuration
    pub(crate) gc: Option<GC>,

    /// Launcher name override
    pub(crate) launcher_name: Option<String>,

    /// Optional command wrapper (e.g., script or proxy)
    pub wrap_command: Option<String>,

    /// Script or command to execute before launch
    pub execute_before_launch: Option<String>,

    /// Script or command to execute after launch
    pub execute_after_launch: Option<String>,
}

/// Main configuration structure for a Minecraft instance.
#[derive(Deserialize, Serialize, Default)]
pub struct InstanceConfig {
    /// Instance name (displayed to user)
    pub name: String,

    /// Minecraft runtime configuration
    pub runtime: InstanceRuntime,

    /// Optional tags or groupings for the instance
    #[serde(default)]
    pub group: Option<Vec<String>>,

    /// Instance-specific launch configuration
    #[serde(default)]
    pub launch_config: InstanceLaunchConfig,
}

impl InstanceConfig {
    /// Creates a new instance configuration with the specified name and Minecraft version.
    ///
    /// # Arguments
    ///
    /// * `instance_name` - The name of the instance.
    /// * `minecraft_version` - The version of Minecraft to use.
    ///
    /// # Returns
    ///
    /// A fully initialized `InstanceConfig` with default settings.
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
