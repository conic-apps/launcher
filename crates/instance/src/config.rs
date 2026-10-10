// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use serde::{Deserialize, Serialize};

use config::launch::{GC, Server};

#[derive(Clone, Deserialize, Serialize)]
pub enum ModLoaderType {
    Fabric,
    Quilt,
    Forge,
    Neoforge,
}

impl fmt::Display for ModLoaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fabric => write!(f, "Fabric"),
            Self::Quilt => write!(f, "Quilt"),
            Self::Forge => write!(f, "Forge"),
            Self::Neoforge => write!(f, "Neoforge"),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct InstanceRuntime {
    pub minecraft: String,

    pub mod_loader_type: Option<ModLoaderType>,

    pub mod_loader_version: Option<String>,
}

/// How this instance launches; its fields override the global launch config.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct InstanceLaunchConfig {
    pub enable_instance_specific_settings: bool,

    /// Overrides the built-in Java environment.
    pub java_path: Option<String>,

    pub auto_memory: Option<bool>,

    /// Maximum heap in MB; emitted to the JVM as `-Xmx`.
    pub max_memory: Option<usize>,

    pub server: Option<Server>,

    pub width: Option<usize>,

    pub height: Option<usize>,

    pub fullscreen: Option<bool>,

    pub extra_jvm_args: Option<String>,

    pub extra_mc_args: Option<String>,

    pub is_demo: Option<bool>,

    /// Adds `-Dfml.ignoreInvalidMinecraftCertificates=true` to the JVM args.
    pub ignore_invalid_minecraft_certificates: Option<bool>,

    /// Adds `-Dfml.ignorePatchDiscrepancies=true` to the JVM args.
    pub ignore_patch_discrepancies: Option<bool>,

    pub extra_class_paths: Option<String>,

    pub gc: Option<GC>,

    pub launcher_name: Option<String>,

    pub wrap_command: Option<String>,

    pub execute_before_launch: Option<String>,

    pub execute_after_launch: Option<String>,

    pub skip_check_files: Option<bool>,

    pub quit_app_after_launch: Option<bool>,
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct InstanceConfig {
    pub name: String,

    pub icon: Option<String>,

    pub runtime: InstanceRuntime,

    #[serde(default)]
    pub group: Option<Vec<String>>,

    #[serde(default)]
    pub launch_config: InstanceLaunchConfig,

    #[serde(default)]
    pub use_as_launcher_background: bool,

    /// How much this instance's background image is darkened, as a percentage,
    /// when it is the launcher background. The global background has its own
    /// `appearance.background_darkness`; an instance background is dimmed at
    /// this value instead, so the two never share a setting.
    #[serde(default)]
    pub background_darkness: u8,
}

impl InstanceConfig {
    pub fn new(instance_name: &str, minecraft_version: &str) -> Self {
        Self {
            name: instance_name.to_string(),
            icon: None,
            runtime: InstanceRuntime {
                minecraft: minecraft_version.to_string(),
                mod_loader_type: None,
                mod_loader_version: None,
            },
            group: None,
            launch_config: InstanceLaunchConfig::default(),
            use_as_launcher_background: false,
            background_darkness: 0,
        }
    }
}
