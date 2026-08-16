// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use account::Account;
use config::{
    Config,
    launch::{GC, Server},
};
use folder::DATA_LOCATION;
use instance::Instance;
use java_runtime::JavaArch;
use log::info;

use crate::error::*;

/// Represents all launch options required to start a Minecraft instance.
///
/// These include memory settings, screen resolution, authentication tokens,
/// optional server connection info, and custom JVM/MC arguments.
pub struct LaunchOptions {
    pub selected_account: Account,

    pub user_properties: String,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub max_memory: usize,

    /// Young generation size in MB, this will add a jvm flag -Xmn to the
    /// command result. Only set when auto memory allocation is used.
    pub xmn_memory: usize,

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

    pub gc: GC,

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
    /// `java_arch` is the architecture of the Java runtime that will be used to
    /// launch the game; a 32-bit runtime caps the auto-allocated heap at 1 GiB.
    pub fn new(config: &Config, instance: &Instance, java_arch: JavaArch) -> Result<Self> {
        let global_launch_config = config.launch.clone();
        let launch_config = &instance.config.launch_config;
        let selected_account = match config.current_account.clone() {
            None => return Err(Error::InvalidProfile),
            Some(account) => account,
        };
        let auto_memory = launch_config
            .auto_memory
            .unwrap_or(global_launch_config.auto_memory);
        let is_32_bit = is_32_bit_java(java_arch);
        let (max_memory, xmn_memory) = if auto_memory {
            let available = platform::get_available_memory_bytes();
            let mod_count = count_instance_mods(instance);
            let (max_memory, xmn_memory) = auto_allocate_memory(
                available,
                instance_has_mod_loader(instance),
                mod_count,
                is_32_bit,
            );
            info!(
                "Auto memory allocation: -Xmx{max_memory}M -Xmn{xmn_memory}M \
                 (available {} MiB, mod count {mod_count}, {} Java)",
                available / 1024 / 1024,
                if is_32_bit { "32-bit" } else { "64-bit" }
            );
            (max_memory, xmn_memory)
        } else {
            let max_memory = launch_config
                .max_memory
                .unwrap_or(global_launch_config.max_memory);
            info!("Manual memory allocation: -Xmx{max_memory}M");
            (max_memory, 0)
        };
        Ok(Self {
            selected_account,
            max_memory,
            xmn_memory,
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
            gc: launch_config.gc.clone().unwrap_or(global_launch_config.gc),
            user_properties: "{}".to_string(),
        })
    }
}

/// Returns whether the instance has a mod loader installed.
fn instance_has_mod_loader(instance: &Instance) -> bool {
    instance.config.runtime.mod_loader_type.is_some()
}

/// Returns whether the Java runtime architecture is 32-bit.
fn is_32_bit_java(arch: JavaArch) -> bool {
    matches!(arch, JavaArch::X86 | JavaArch::Arm)
}

/// Counts the number of mod files in the instance's `mods` directory.
///
/// This is a rough estimate based on the file count only, no archive parsing.
fn count_instance_mods(instance: &Instance) -> usize {
    let mods_folder = DATA_LOCATION.get_instance_root(&instance.id).join("mods");
    match std::fs::read_dir(&mods_folder) {
        Ok(entries) => entries
            .flatten()
            .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
            .count(),
        Err(_) => 0,
    }
}

/// Calculates the maximum heap (`-Xmx`) and young generation (`-Xmn`) memory
/// in MB, following the same algorithm PCL uses for its auto allocation.
///
/// # Arguments
///
/// * `available_bytes` - The currently available physical memory in bytes.
/// * `has_mod_loader` - Whether the instance supports mods.
/// * `mod_count` - Number of mod files in the instance's `mods` directory.
/// * `is_32_bit` - Whether the Java runtime is 32-bit; the heap is then capped
///   at 1 GiB because a 32-bit JVM cannot address much more.
fn auto_allocate_memory(
    available_bytes: u64,
    has_mod_loader: bool,
    mod_count: usize,
    is_32_bit: bool,
) -> (usize, usize) {
    let mut available = (available_bytes as f64 / 1073741824.0 * 10.0).round() / 10.0;

    let (ram_minimum, target1, target2, target3) = if has_mod_loader {
        let mod_count = mod_count as f64;
        (
            0.5 + mod_count / 150.0,
            1.5 + mod_count / 90.0,
            2.7 + mod_count / 50.0,
            4.5 + mod_count / 25.0,
        )
    } else {
        (0.5, 1.5, 2.5, 4.0)
    };

    let stages = [
        (target1, 1.0),
        (target2 - target1, 0.7),
        (target3 - target2, 0.4),
        (target3, 0.15),
    ];

    let mut ram_give = 0.0;
    for (delta, ratio) in stages {
        ram_give += (available * ratio).min(delta);
        available -= delta / ratio;
        if available < 0.1 {
            break;
        }
    }
    let mut ram_give = (ram_give.max(ram_minimum) * 10.0).round() / 10.0;
    if is_32_bit {
        ram_give = ram_give.min(1.0);
    }

    let max_memory = (ram_give * 1024.0).floor() as usize;
    let xmn_memory = (ram_give * 1024.0 * 0.15).floor() as usize;
    (max_memory, xmn_memory)
}
