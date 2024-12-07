// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::BufRead,
    process::{Command, Stdio},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use arguments::generate_command_arguments;
use complete::complete_files;
use log::{error, info, trace};
use options::LaunchOptions;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
mod arguments;
mod complete;
mod options;
use crate::{
    account::{self, refresh_microsoft_account_by_uuid, Account},
    folder::MinecraftLocation,
    instance::Instance,
    platform::OsType,
    version::Version,
    Storage, DATA_LOCATION, MAIN_WINDOW, PLATFORM_INFO,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    #[serde(rename = "instanceName")]
    pub instance_name: String,
    pub content: String,
}

async fn check_and_refresh_account(account: &Account) -> anyhow::Result<Account> {
    info!("Checking account: {}", account.profile.uuid);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    const AHEAD: u64 = 3600 * 4;
    let token_deadline = match account.token_deadline {
        Some(x) => x,
        None => return Ok(account.clone()),
    };
    if now > token_deadline - AHEAD {
        info!("The access token will expire in 4 hours");
        let refreshed_account =
            refresh_microsoft_account_by_uuid(account.profile.uuid.to_string()).await;
        Ok(refreshed_account)
    } else {
        info!(
            "The access token will expire in {} seconds, no need to refresh.",
            token_deadline - now
        );
        Ok(account.clone())
    }
}

#[tauri::command(async)]
pub async fn launch(storage: tauri::State<'_, Storage>, instance: Instance) -> Result<(), ()> {
    info!(
        "Starting Minecraft client, instance: {}",
        instance.config.name
    );
    let platform = PLATFORM_INFO.get().unwrap();
    let instance_config = instance.config;
    info!("------------- Instance runtime config -------------");
    info!("-> Minecraft: {}", instance_config.runtime.minecraft);
    match &instance_config.runtime.mod_loader_type {
        Some(x) => info!("-> Mod loader: {x}"),
        None => info!("-> Mod loader: none"),
    };
    match &instance_config.runtime.mod_loader_version {
        Some(x) => info!("-> Mod loader version: {x}"),
        None => info!("-> Mod loader version: none"),
    };
    let config = storage.config.lock().unwrap().clone();
    let selected_account = account::get_account_by_uuid(&config.current_account);
    let selected_account = match selected_account.first() {
        Some(x) => x,
        None => {
            error!("The selected account not been found, opening account manager");
            MAIN_WINDOW
                .get()
                .unwrap()
                .emit("add-account", "add-account")
                .unwrap();
            return Err(());
        }
    };
    let selected_account = if config.launch.skip_refresh_account {
        info!("Account refresh disabled by user");
        selected_account.clone()
    } else {
        check_and_refresh_account(selected_account).await.unwrap()
    };

    let launch_options = LaunchOptions::get(&instance_config, selected_account);
    let minecraft_location = launch_options.minecraft_location.clone();
    if config.launch.skip_check_files {
        info!("File checking disabled by user")
    } else {
        complete_files(&instance_config, &minecraft_location).await;
    }

    info!("Generating startup parameters");
    let version =
        Version::from_versions_folder(&minecraft_location, &instance_config.get_version_id())
            .unwrap()
            .parse(&minecraft_location, platform)
            .await
            .unwrap();
    let command_arguments = generate_command_arguments(
        &minecraft_location,
        &instance_config,
        platform,
        &launch_options,
        version.clone(),
    )
    .await;
    let version_id = version.id;
    thread::spawn(move || {
        spawn_minecraft_process(
            command_arguments,
            minecraft_location,
            launch_options,
            version_id,
            &instance_config.name,
        )
    });
    Ok(())
}

fn spawn_minecraft_process(
    command_arguments: Vec<String>,
    minecraft_location: MinecraftLocation,
    launch_options: LaunchOptions,
    version_id: String,
    instance_name: &str,
) {
    let platform = PLATFORM_INFO.get().unwrap();
    let native_root = minecraft_location.get_natives_root(&version_id);
    let instance_root = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(instance_name);
    let mut commands = String::new();
    if platform.os_type == OsType::Linux {
        commands.push_str("#!/bin/bash\n\n");
    }
    let comment_prefix = match platform.os_type {
        OsType::Windows => "::",
        _ => "#",
    };
    commands.push_str(&format!(
        "{comment_prefix} This file is automatically generated by Amethyst Launcher.\n"
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
        launch_command = format!("{}{}", launch_command, arg);
    }
    commands.push_str(&launch_command);
    match platform.os_type {
        OsType::Windows => {
            commands.push_str(&format!("\ndel /F /Q {}\n", native_root.to_string_lossy()))
        }
        _ => commands.push_str(&format!("\nrm -rf {}\n", native_root.to_string_lossy())),
    }
    commands.push_str(&format!("{}\n", launch_options.execute_after_launch));
    let script_path = match platform.os_type {
        OsType::Linux => instance_root.join(".cache").join("launch.sh"),
        OsType::Osx => instance_root.join(".cache").join("launch.sh"),
        OsType::Windows => instance_root.join(".cache").join("launch.bat"),
    };

    std::fs::create_dir_all(script_path.parent().unwrap()).unwrap();
    std::fs::write(&script_path, commands).unwrap();
    info!("The startup script is written to {}", script_path.display());
    let mut minecraft_process = match platform.os_type {
        OsType::Windows => std::process::Command::new(script_path),
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
    let main_window = MAIN_WINDOW.get().unwrap();
    let id = minecraft_process.id();
    while out.read_line(&mut buf).is_ok() {
        if let Ok(Some(_)) = minecraft_process.try_wait() {
            break;
        }
        let lines: Vec<_> = buf.split("\n").collect();
        if let Some(last) = lines.get(lines.len() - 2) {
            trace!("[{}] {}", id, last);
            if last.to_lowercase().contains("lwjgl version") {
                main_window.emit("launch_success", instance_name).unwrap();
                info!("Found LWJGL version, the game seems to have started successfully.");
            }
            main_window
                .emit(
                    "log",
                    Log {
                        instance_name: instance_name.to_string(),
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
