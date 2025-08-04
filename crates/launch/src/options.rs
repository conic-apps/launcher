// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use account::Account;
use config::{
    Config,
    launch::{GC, Server},
};
use folder::DATA_LOCATION;
use folder::MinecraftLocation;
use instance::Instance;

/// Represents a game profile used for launching Minecraft.
///
/// Contains player name and UUID.
pub struct GameProfile {
    /// The player's in-game name.
    pub name: String,

    /// The UUID associated with the game profile.
    pub uuid: String,
}

/// Represents all launch options required to start a Minecraft instance.
///
/// These include memory settings, screen resolution, authentication tokens,
/// optional server connection info, and custom JVM/MC arguments.
pub struct LaunchOptions {
    /// User selected game profile.
    ///
    /// For game display name & uuid
    pub game_profile: GameProfile,

    pub properties: String,
    pub access_token: String,

    /// Min memory, this will add a jvm flag -XMS to the command result
    pub min_memory: usize,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub max_memory: usize,

    /// Enter a server after launch. TODO: support 1.21.1
    pub server: Option<Server>,

    /// window width
    pub width: usize,

    /// window height
    pub height: usize,

    pub fullscreen: bool,

    /// User custom additional java virtual machine command line arguments.
    ///
    /// If this is empty, the `DEFAULT_EXTRA_JVM_ARGS` will be used.
    pub extra_jvm_args: String,

    /// User custom additional minecraft command line arguments.
    pub extra_mc_args: String,

    /// Launch game in demo mode, I don't know who want it:)
    /// NOTE: Should NOT allow user who don't have the game access this launcher because legal
    /// issues
    pub is_demo: bool,

    /// Adds `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub ignore_invalid_minecraft_certificates: bool,

    /// Adds `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub ignore_patch_discrepancies: bool,

    /// Adds extra classpath
    pub extra_class_paths: String,

    /// Adds other features flags
    pub extra_enabled_features: Vec<String>,

    // /// TODO: Support yushi's yggdrasil agent <https://github.com/to2mbn/authlib-injector/wiki>
    // pub yggdrasil_agent: Option<YggdrasilAgent>,
    pub gc: GC,

    pub minecraft_location: MinecraftLocation,
    pub launcher_name: String,

    /// Optional command used to wrap the final launch command.
    pub wrap_command: String,

    /// Shell command to execute before the game launches.
    pub execute_before_launch: String,

    /// Shell command to execute after the game exits.
    pub execute_after_launch: String,
}

impl LaunchOptions {
    /// Creates a new [`LaunchOptions`] instance from the given Minecraft instance and account.
    ///
    /// Launch configuration is resolved from both global and per-instance settings,
    /// with per-instance settings taking priority when defined.
    ///
    /// # Arguments
    /// * `instance` - The Minecraft instance to launch.
    /// * `account` - The account used to authenticate the session.
    ///
    /// # Returns
    /// A fully populated `LaunchOptions` struct used for launching the game.
    pub fn new(config: &Config, instance: &Instance, account: Account) -> Self {
        let global_launch_config = config.launch.clone();
        let launch_config = &instance.config.launch_config;
        Self {
            wrap_command: launch_config
                .wrap_command
                .clone()
                .unwrap_or(global_launch_config.wrap_command),
            execute_before_launch: launch_config
                .execute_before_launch
                .clone()
                .unwrap_or(global_launch_config.execute_before_launch),
            execute_after_launch: launch_config
                .execute_after_launch
                .clone()
                .unwrap_or(global_launch_config.execute_after_launch),
            launcher_name: launch_config
                .launcher_name
                .clone()
                .unwrap_or(global_launch_config.launcher_name),
            game_profile: GameProfile {
                name: account.profile.profile_name.clone(),
                uuid: account.profile.uuid.clone(),
            },
            access_token: account.access_token.clone().unwrap_or_default(),
            min_memory: launch_config
                .min_memory
                .unwrap_or(global_launch_config.min_memory),
            max_memory: launch_config
                .max_memory
                .unwrap_or(global_launch_config.max_memory),
            // TODO:
            server: launch_config.server.clone(),
            width: launch_config.width.unwrap_or(global_launch_config.width),
            height: launch_config.height.unwrap_or(global_launch_config.height),
            fullscreen: launch_config
                .fullscreen
                .unwrap_or(global_launch_config.fullscreen),
            extra_jvm_args: launch_config
                .extra_jvm_args
                .clone()
                .unwrap_or(global_launch_config.extra_jvm_args),
            extra_mc_args: launch_config
                .extra_mc_args
                .clone()
                .unwrap_or(global_launch_config.extra_mc_args),
            is_demo: launch_config
                .is_demo
                .unwrap_or(global_launch_config.is_demo),
            ignore_invalid_minecraft_certificates: launch_config
                .ignore_invalid_minecraft_certificates
                .unwrap_or(global_launch_config.ignore_invalid_minecraft_certificates),
            ignore_patch_discrepancies: launch_config
                .ignore_patch_discrepancies
                .unwrap_or(global_launch_config.ignore_patch_discrepancies),
            extra_class_paths: launch_config
                .extra_class_paths
                .clone()
                .unwrap_or(global_launch_config.extra_class_paths),
            extra_enabled_features: vec![],
            gc: launch_config.gc.clone().unwrap_or(global_launch_config.gc),
            minecraft_location: MinecraftLocation::new(&DATA_LOCATION.root),
            properties: "{}".to_string(),
        }
    }

    /// Returns a list of enabled feature flags for the current launch configuration.
    ///
    /// By default, includes `has_custom_resolution`, and may include:
    /// - `is_demo_user` if `is_demo` is true
    /// - Any features listed in `extra_enabled_features`
    pub fn get_enabled_features(&self) -> Vec<String> {
        let mut result = vec!["has_custom_resolution".to_string()];
        if self.is_demo {
            result.push("is_demo_user".to_string())
        }
        result.extend(self.extra_enabled_features.clone());
        result
    }
}
