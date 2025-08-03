// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{HashMap, HashSet};

use log::info;
use regex::Regex;
use zip::ZipArchive;

use crate::{
    config::launch::GC,
    folder::MinecraftLocation,
    instance::Instance,
    platform::{OsFamily, DELIMITER},
    utils::unzip::decompression_all,
    version::ResolvedVersion,
    APP_VERSION, DATA_LOCATION, PLATFORM_INFO,
};

use super::options::LaunchOptions;

const DEFAULT_GAME_ICON: &[u8] = include_bytes!("../../assets/minecraft.icns");

pub async fn generate_command_arguments(
    minecraft_location: &MinecraftLocation,
    instance: &Instance,
    launch_options: &LaunchOptions,
    version: ResolvedVersion,
) -> Vec<String> {
    let mut command_arguments = Vec::new();

    command_arguments.push(format!(
        "\"-Dminecraft.client.jar={version_jar}\"",
        version_jar = minecraft_location
            .get_version_jar(&instance.config.runtime.minecraft, None)
            .to_string_lossy()
    ));
    let game_icon = minecraft_location
        .assets
        .join("minecraft.icns")
        .to_string_lossy()
        .to_string();
    tokio::fs::write(&game_icon, DEFAULT_GAME_ICON)
        .await
        .unwrap();
    if PLATFORM_INFO.os_family == OsFamily::Macos {
        command_arguments.push("-Xdock:name=Minecraft".to_string());
        command_arguments.push(format!(
            "-Xdock:icon={game_icon}",
            game_icon = if game_icon.contains(" ") {
                format!("\"{game_icon}\"")
            } else {
                game_icon
            }
        ));
    }
    if launch_options.min_memory > 0 {
        command_arguments.push(format!("-Xms{}M", launch_options.min_memory));
    }
    if launch_options.max_memory > 0 {
        command_arguments.push(format!("-Xmx{}M", launch_options.max_memory));
    }
    if launch_options.ignore_invalid_minecraft_certificates {
        command_arguments.push("-Dfml.ignoreInvalidMinecraftCertificates=true".to_string());
    }
    if launch_options.ignore_patch_discrepancies {
        command_arguments.push("-Dfml.ignorePatchDiscrepancies=true".to_string());
    }
    match launch_options.gc {
        GC::G1 => {
            command_arguments.extend([
                "-XX:+UseG1GC".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=16M".to_string(),
            ]);
        }
        GC::Parallel => {
            command_arguments.extend([
                "-XX:+UseParallelGC".to_string(),
                format!(
                    "-XX:ParallelGCThreads={num}",
                    num = num_cpus::get_physical()
                ),
            ]);
        }
        GC::ParallelOld => {
            command_arguments.push("-XX:+UseParallelOldGC".to_string());
        }
        GC::Serial => {
            command_arguments.push("-XX:+UseSerialGC".to_string());
        }
        GC::Z => {
            command_arguments.push("-XX:+UseZGC".to_string());
        }
    }
    // TODO: support yggdrasil
    //         if let Some(ygg) = launch_options.yggdrasil_agent.clone() {
    //             command_arguments.push(format!(
    //                 "-javaagent:{jar}={server}",
    //                 jar = ygg.jar.to_string_lossy(),
    //                 server = ygg.server
    //             ));
    //             command_arguments.push("-Dauthlibinjector.side=client".to_string());
    //             if let Some(prefetched) = ygg.prefetched {
    //                 command_arguments.push(format!(
    //                     "-Dauthlibinjector.yggdrasil.prefetched={prefetched}"
    //                 ));
    //             }
    //         }
    let mut jvm_options: HashMap<&str, String> = HashMap::new();
    jvm_options.insert(
        "natives_directory",
        minecraft_location
            .get_natives_root(&version.id)
            .to_string_lossy()
            .to_string(),
    );
    jvm_options.insert("launcher_name", launch_options.launcher_name.clone());
    jvm_options.insert("launcher_version", APP_VERSION.get().unwrap().to_string());
    jvm_options.insert(
        "classpath",
        resolve_classpath(
            &version,
            minecraft_location,
            launch_options.extra_class_paths.clone(),
        ),
    );
    jvm_options.insert("version_name", version.id.clone());
    jvm_options.insert(
        "library_directory",
        DATA_LOCATION.root.join("libraries").display().to_string(),
    );
    let mut jvm_arguments = Vec::new();
    if let Some(client) = version.logging.get("client") {
        let argument = &client.argument;
        let file_path = minecraft_location
            .get_version_root(&version.id)
            .join("log4j2.xml");
        if tokio::fs::try_exists(&file_path).await.unwrap() {
            jvm_arguments.push(format!(
                "\"{}\"",
                argument.replace("${path}", file_path.to_string_lossy().as_ref())
            ));
        }
    }
    jvm_arguments.extend(version.arguments.jvm);
    command_arguments.push(launch_options.extra_jvm_args.clone());
    command_arguments.extend(
        jvm_arguments
            .iter()
            .map(|arg| format(arg, jvm_options.clone(), false)),
    );
    command_arguments.push(
        version
            .main_class
            .unwrap_or("net.minecraft.client.main.Main".to_string()),
    );
    let mut game_options: HashMap<&str, String> = HashMap::with_capacity(13);
    let assets_dir = minecraft_location.assets.clone();
    game_options.insert("version_name", version.id.clone());
    game_options.insert("version_type", launch_options.launcher_name.clone());
    game_options.insert("assets_root", assets_dir.to_string_lossy().to_string());
    game_options.insert(
        "game_assets",
        assets_dir
            .join("virtual")
            .join(version.assets.as_ref().unwrap())
            .to_string_lossy()
            .to_string(),
    );
    game_options.insert("asset_index", version.asset_index.unwrap().id);
    game_options.insert("assets_index_name", version.assets.unwrap());
    game_options.insert(
        "game_directory",
        DATA_LOCATION
            .get_instance_root(&instance.id)
            .to_string_lossy()
            .to_string(),
    );
    game_options.insert("auth_player_name", launch_options.game_profile.name.clone());
    game_options.insert("auth_uuid", launch_options.game_profile.uuid.clone());
    game_options.insert("auth_access_token", launch_options.access_token.clone());
    game_options.insert("user_properties", launch_options.properties.clone());
    game_options.insert("user_type", "msa".to_string());
    game_options.insert("resolution_width", launch_options.width.to_string());
    game_options.insert("resolution_height", launch_options.height.to_string());
    command_arguments.extend(
        version
            .arguments
            .game
            .iter()
            .map(|arg| format(arg, game_options.clone(), true)),
    );
    command_arguments.push(launch_options.extra_mc_args.clone());
    if let Some(server) = launch_options.server.clone() {
        command_arguments.extend(vec!["--server".to_string(), server.ip]);
        if let Some(port) = server.port {
            command_arguments.extend(vec!["--port".to_string(), port.to_string()])
        }
    }
    if launch_options.fullscreen {
        command_arguments.push("--fullscreen".to_string());
    }
    let no_width_arguments = !command_arguments
        .iter()
        .any(|v| v == &"--width".to_string());
    if no_width_arguments && !launch_options.fullscreen {
        command_arguments.extend(vec![
            "--width".to_string(),
            launch_options.width.to_string(),
            "--height".to_string(),
            launch_options.height.to_string(),
        ]);
    }
    if launch_options.is_demo {
        command_arguments.push("--demo".to_string());
    };
    command_arguments
}

fn resolve_classpath(
    version: &ResolvedVersion,
    minecraft: &MinecraftLocation,
    extra_class_paths: String,
) -> String {
    let mut classpath = version
        .libraries
        .iter()
        .filter(|lib| {
            if lib.is_native_library {
                let path = minecraft.get_library_by_path(&lib.download_info.path);
                let native_folder = minecraft.get_natives_root(&version.id);
                info!("Unzip native library {:#?} to {:#?}", path, native_folder);
                if let Ok(file) = std::fs::File::open(path) {
                    if let Ok(mut zip_archive) = ZipArchive::new(file) {
                        decompression_all(&mut zip_archive, &native_folder).unwrap_or(());
                    }
                }
            }
            !lib.is_native_library
        })
        .map(|lib| {
            minecraft
                .get_library_by_path(lib.download_info.path.clone())
                .to_string_lossy()
                .to_string()
        })
        .collect::<HashSet<String>>()
        .into_iter()
        .collect::<Vec<String>>();

    if !extra_class_paths.is_empty() {
        classpath.push(extra_class_paths);
    }

    if let Some(inheritance) = version.inheritances.last() {
        classpath.push(
            minecraft
                .get_version_jar(inheritance, None)
                .to_string_lossy()
                .to_string(),
        );
    } else {
        classpath.push(
            minecraft
                .get_version_jar(&version.id, None)
                .to_string_lossy()
                .to_string(),
        );
    }

    classpath.join(DELIMITER)
}

fn format(template: &str, args: HashMap<&str, String>, is_game_option: bool) -> String {
    let regex = Regex::new(r"\$\{(.*?)}").unwrap();

    regex
        .replace_all(template, |caps: &regex::Captures| {
            let key = String::from(&caps[1]);
            let value = args.get(&caps[1]).unwrap_or(&key);
            if value.contains(" ") && is_game_option {
                format!("\"{value}\"")
            } else {
                value.to_string()
            }
        })
        .to_string()
}
