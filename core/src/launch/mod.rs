// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::BufRead,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

use arguments::generate_command_arguments;
use complete::complete_files;
use log::{debug, error, info};
use options::LaunchOptions;
mod arguments;
mod complete;
mod options;
use crate::{
    config::{instance::InstanceConfig, launch::ProcessPriority},
    folder::MinecraftLocation,
    platform::OsType,
    version::Version,
    PLATFORM_INFO,
};

#[tauri::command(async)]
pub async fn launch(instance_name: String) {
    println!("Starting Minecraft client, instance: {}", instance_name);
    let platform = PLATFORM_INFO.get().unwrap();
    let instance = InstanceConfig::get(&instance_name).await.unwrap();
    info!("------------- Instance runtime config -------------");
    info!("-> Minecraft: {}", instance.runtime.minecraft);
    match &instance.runtime.mod_loader_type {
        Some(x) => info!("-> Mod loader: {x}"),
        None => info!("-> Mod loader: none"),
    };
    match &instance.runtime.mod_loader_version {
        Some(x) => info!("-> Mod loader version: {x}"),
        None => info!("-> Mod loader version: none"),
    };
    let launch_options = LaunchOptions::get(instance.clone());
    let minecraft_location = launch_options.minecraft_location.clone();
    complete_files(instance.clone(), minecraft_location.clone()).await;
    info!("Generating startup parameters");
    let version = Version::from_versions_folder(&minecraft_location, &instance.get_version_id())
        .unwrap()
        .parse(&minecraft_location, platform)
        .await
        .unwrap();
    let command_arguments = generate_command_arguments(
        &minecraft_location,
        instance,
        platform,
        &launch_options,
        version.clone(),
    )
    .await;
    let version_id = version.id;
    let process_priority = launch_options.process_priority;
    thread::spawn(move || {
        spawn_minecraft_process(
            command_arguments,
            minecraft_location,
            process_priority,
            version_id,
        )
    });
}

fn spawn_minecraft_process(
    command_arguments: Vec<String>,
    minecraft_location: MinecraftLocation,
    process_priority: ProcessPriority,
    version_id: String,
) {
    let platform = PLATFORM_INFO.get().unwrap();
    let native_root = minecraft_location.get_natives_root(&version_id);
    let version_root = minecraft_location.get_version_root(version_id);
    let mut command = format!("cd {}\n", version_root.to_string_lossy());
    match platform.os_type {
        OsType::Windows => {}
        _ => command.push_str("nice "),
    };

    if let OsType::Windows = platform.os_type {
        command.push_str(" -c ");
    }

    if platform.os_type != OsType::Windows {
        match process_priority {
            ProcessPriority::High => {
                command.push_str("-n 0 ");
            }
            ProcessPriority::AboveNormal => {
                command.push_str("-n 5 ");
            }
            ProcessPriority::Normal => (), // nothing to do
            ProcessPriority::BelowNormal => {
                command.push_str("-n 15 ");
            }
            ProcessPriority::Low => {
                command.push_str("-n 19 ");
            }
        };
    }
    // todo(after java exec): add -Dfile.encoding=encoding.name() and other
    let mut launch_command =
        "/usr/lib/jvm/java-21-openjdk-21.0.4.0.7-2.fc40.x86_64/bin/java".to_string();
    for arg in command_arguments.clone() {
        launch_command.push(' ');
        launch_command = format!("{}{}", launch_command, arg);
    }
    command.push_str(&launch_command);
    match platform.os_type {
        OsType::Windows => {
            command.push_str(&format!("\ndel /F /Q {}\n", native_root.to_string_lossy()))
        }
        _ => command.push_str(&format!("\nrm -rf {}", native_root.to_string_lossy())),
    }
    let script_path = match platform.os_type {
        OsType::Linux => version_root.join(".cache").join("launch.sh"),
        OsType::Osx => version_root.join(".cache").join("launch.sh"),
        OsType::Windows => version_root.join(".cache").join("launch.bat"),
    };

    std::fs::create_dir_all(script_path.parent().unwrap()).unwrap();
    std::fs::write(&script_path, command).unwrap();
    info!("The startup script is written to {}", script_path.display());
    let mut minecraft = match platform.os_type {
        OsType::Windows => {
            let vars = std::env::vars().find(|v| v.0 == "PATH").unwrap();

            let path_vars = vars.1.as_str().split(";").collect::<Vec<&str>>(); // todo: test it in windows
            let powershell_folder = PathBuf::from(
                path_vars
                    .into_iter()
                    .find(|v| v.to_lowercase().contains("powershell"))
                    .unwrap(),
            );

            std::process::Command::new(
                powershell_folder
                    .join("powershell.exe")
                    .to_string_lossy()
                    .to_string(),
            )
        }
        _ => {
            info!("Running chmod +x {}", script_path.display());
            let mut chmod = Command::new("chmod");
            chmod.args(["+x", script_path.to_string_lossy().to_string().as_ref()]);
            chmod.status().unwrap();
            std::process::Command::new("bash")
        }
    }
    .arg(script_path)
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
    info!("Spawning minecraft process");
    let out = minecraft.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    while out.read_line(&mut buf).is_ok() {
        if let Ok(Some(_)) = minecraft.try_wait() {
            break;
        }
        let lines: Vec<_> = buf.split("\n").collect();
        if let Some(last) = lines.get(lines.len() - 2) {
            debug!("{}", last);
        }
    }
    let output = minecraft.wait_with_output().unwrap();
    if !output.status.success() {
        // TODO: log analysis and remove libraries lock file
        error!("Minecraft exits with error code {}", output.status);
    } else {
        info!("Minecraft exits with error code {}", output.status);
    }
}
