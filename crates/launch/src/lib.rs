// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::BufRead,
    process::{Command, Stdio},
    str::FromStr,
    thread,
};

use account::check_and_refresh_account;
use arguments::generate_command_arguments;
use complete::complete_files;
use config::Config;
use folder::{DATA_LOCATION, MinecraftLocation};
use instance::Instance;
use log::{error, info, trace};
use options::LaunchOptions;
use platform::{OsFamily, PLATFORM_INFO};
use serde::Serialize;
use shared::MAIN_WINDOW;
use tauri::{
    Emitter, Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;
use version::Version;

mod arguments;
mod complete;
mod options;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("launch")
        .invoke_handler(tauri::generate_handler![cmd_launch])
        .build()
}

#[command]
async fn cmd_launch(config: Config, instance: Instance) {
    let _ = launch(config, instance).await;
}

/// Represents a log message associated with a specific instance.
#[derive(Clone, Serialize)]
pub struct Log {
    /// The UUID of the instance this log belongs to.
    #[serde(rename = "instanceName")]
    pub instance_id: Uuid,

    /// The content of the log message.
    pub content: String,
}

/// Launches a Minecraft instance asynchronously via the Tauri command system.
///
/// # Arguments
/// * `storage` - Application state that holds shared configuration and data.
/// * `instance` - The Minecraft instance to launch.
///
/// # Returns
/// * `Ok(())` - If the instance was successfully launched.
/// * `Err(())` - If there was an error during launch (e.g., account not found).
///
/// # Side Effects
/// * Refreshes the selected account if configured to do so.
/// * Optionally checks files before launch.
/// * Spawns the Minecraft process and generates launch script.
pub async fn launch(config: Config, instance: Instance) -> Result<(), ()> {
    info!(
        "Starting Minecraft client, instance: {}",
        instance.config.name
    );
    info!("------------- Instance runtime config -------------");
    info!("-> Minecraft: {}", instance.config.runtime.minecraft);
    match &instance.config.runtime.mod_loader_type {
        Some(x) => info!("-> Mod loader: {x}"),
        None => info!("-> Mod loader: none"),
    };
    match &instance.config.runtime.mod_loader_version {
        Some(x) => info!("-> Mod loader version: {x}"),
        None => info!("-> Mod loader version: none"),
    };

    if !config.launch.skip_refresh_account {
        check_and_refresh_account(config.current_account_uuid, &config.current_account_type)
            .await
            .unwrap();
    } else {
        info!("Account refresh disabled by user");
    };
    let selected_account =
        account::AccountLaunchInfo::new(config.current_account_uuid, &config.current_account_type)
            .unwrap();

    let launch_options = LaunchOptions::new(&config, &instance, &selected_account);
    let minecraft_location = launch_options.minecraft_location.clone();

    if config.launch.skip_check_files {
        info!("File checking disabled by user")
    } else {
        complete_files(&instance, &minecraft_location).await;
    }

    info!("Generating startup parameters");
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id());
    let raw_version_json = async_fs::read_to_string(version_json_path).await.unwrap();
    let resolved_version = Version::from_str(&raw_version_json)
        .unwrap()
        .resolve(&minecraft_location, &[])
        .await
        .unwrap();
    let version_id = resolved_version.id.clone();
    let command_arguments = generate_command_arguments(
        &minecraft_location,
        &instance,
        &launch_options,
        resolved_version,
    )
    .await;
    thread::spawn(move || {
        spawn_minecraft_process(
            command_arguments,
            minecraft_location,
            launch_options,
            version_id,
            instance,
        )
    });
    Ok(())
}

/// Spawns the Minecraft process by generating and executing a launch script,
/// customized per operating system and instance configuration.
///
/// # Arguments
/// * `command_arguments` - A list of parsed arguments.
/// * `minecraft_location` - Path to the Minecraft game files.
/// * `launch_options` - Launch customization options (pre/post-execution hooks, wrappers, etc.).
/// * `version_id` - The Minecraft version to launch.
/// * `instance` - The instance metadata and configuration.
///
/// # Behavior
/// * Creates a platform-specific shell script/batch file for launching the game.
/// * Runs the generated script using a subprocess.
/// * Streams stdout to detect key launch indicators and forward logs to the frontend.
/// * Emits `launch_success` event once LWJGL is detected.
/// * Handles cleanup of native libraries after game launch completes.
fn spawn_minecraft_process(
    command_arguments: Vec<String>,
    minecraft_location: MinecraftLocation,
    launch_options: LaunchOptions,
    version_id: String,
    instance: Instance,
) {
    let native_root = minecraft_location.get_natives_root(&version_id);
    let instance_root = DATA_LOCATION.get_instance_root(&instance.id);
    let mut commands = String::new();
    if PLATFORM_INFO.os_family == OsFamily::Linux {
        commands.push_str("#!/bin/bash\n\n");
    }
    let comment_prefix = if PLATFORM_INFO.os_family == OsFamily::Windows {
        "::"
    } else {
        "#"
    };
    commands.push_str(&format!(
        "{comment_prefix} This file is automatically generated by Conic Launcher.\n"
    ));
    commands.push_str(&format!(
        "{comment_prefix} NOTE: Don't use this file to launch game.\n\n"
    ));
    commands.push_str(&format!("cd \"{}\"\n", instance_root.to_string_lossy()));
    commands.push_str(&format!("{}\n", launch_options.execute_before_launch));
    if !launch_options.wrap_command.trim().is_empty() {
        commands.push_str(&format!("{} ", launch_options.wrap_command));
    }
    // todo(after java exec): add -Dfile.encoding=encoding.name() and other
    let mut launch_command = "java".to_string();
    for arg in command_arguments.clone() {
        launch_command.push(' ');
        launch_command = format!("{launch_command}{arg}");
    }
    commands.push_str(&launch_command);
    if PLATFORM_INFO.os_family == OsFamily::Windows {
        commands.push_str(&format!("\ndel /F /Q {}\n", native_root.to_string_lossy()))
    } else {
        commands.push_str(&format!("\nrm -rf {}\n", native_root.to_string_lossy()))
    };
    commands.push_str(&format!("{}\n", launch_options.execute_after_launch));
    let script_path = match PLATFORM_INFO.os_family {
        OsFamily::Linux => instance_root.join(".cache").join("launch.sh"),
        OsFamily::Macos => instance_root.join(".cache").join("launch.sh"),
        OsFamily::Windows => instance_root.join(".cache").join("launch.bat"),
    };

    std::fs::create_dir_all(script_path.parent().unwrap()).unwrap();
    std::fs::write(&script_path, commands).unwrap();
    info!("The startup script is written to {}", script_path.display());
    let mut minecraft_process = match PLATFORM_INFO.os_family {
        OsFamily::Windows => std::process::Command::new(script_path),
        _ => {
            info!("Running chmod +x {}", script_path.display());
            let mut chmod = Command::new("chmod");
            chmod.args(["+x", script_path.to_string_lossy().to_string().as_ref()]);
            chmod.status().unwrap();
            let mut command = std::process::Command::new("bash");
            command.arg(script_path);
            command
        }
    }
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
    info!("Spawning minecraft process");
    let out = minecraft_process.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let pid = minecraft_process.id();
    while out.read_line(&mut buf).is_ok() {
        if let Ok(Some(_)) = minecraft_process.try_wait() {
            break;
        }
        let lines: Vec<_> = buf.split("\n").collect();
        if let Some(last) = lines.get(lines.len() - 2) {
            trace!("[{pid}] {last}");
            if last.to_lowercase().contains("lwjgl version") {
                MAIN_WINDOW.emit("launch_success", instance.id).unwrap();
                info!("Found LWJGL version, the game seems to have started successfully.");
            }
            MAIN_WINDOW
                .emit(
                    "log",
                    Log {
                        instance_id: instance.id,
                        content: last.to_string(),
                    },
                )
                .unwrap();
        }
    }
    let output = minecraft_process.wait_with_output().unwrap();
    if !output.status.success() {
        // TODO: log analysis and remove libraries lock file
        error!("Minecraft exits with error code {}", output.status);
    } else {
        info!("Minecraft exits with error code {}", output.status);
    }
}
