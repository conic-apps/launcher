// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    ffi::OsStr,
    io::{self, Read},
    path::Path,
};

use log::info;
use regex::Regex;
use zip::ZipArchive;

use config::launch::GC;
use folder::DATA_LOCATION;
use folder::MinecraftLocation;
use instance::Instance;
use platform::PLATFORM_INFO;
use platform::{DELIMITER, OsFamily};
use shared::APP_VERSION;
use version::{ResolvedLibrary, ResolvedVersion};

use super::error::*;
use super::options::LaunchOptions;

const DEFAULT_GAME_ICON: &[u8] = include_bytes!("./minecraft.icns");

/// Generates the full list of command-line arguments to launch Minecraft.
///
/// # Arguments
///
/// * `minecraft_location` - Reference to MinecraftLocation struct for file paths.
/// * `instance` - The game instance configuration and info.
/// * `launch_options` - User specified launch options and settings.
/// * `version` - The resolved Minecraft version data.
///
/// # Returns
///
/// A vector of strings representing the full command-line arguments to pass to the Java launcher.
pub async fn generate_command_arguments(
    minecraft_location: &MinecraftLocation,
    instance: &Instance,
    launch_options: &LaunchOptions,
    version: ResolvedVersion,
) -> Result<Vec<String>> {
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
    async_fs::write(&game_icon, DEFAULT_GAME_ICON).await?;
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
    jvm_options.insert(
        "launcher_version",
        APP_VERSION.get().cloned().unwrap_or("0.0.0".to_string()),
    );
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
    let mut jvm_arguments = Vec::with_capacity(version.jvm_arguments.len() + 1);
    let log_config_path = minecraft_location.get_log_config(&version.id);
    if let Some(client) = version.logging.get("client")
        && async_fs::metadata(&log_config_path).await.is_ok()
    {
        let argument = &client.argument;
        jvm_arguments.push(format!(
            "\"{}\"",
            argument.replace("${path}", log_config_path.to_string_lossy().as_ref())
        ));
    }
    jvm_arguments.extend(version.jvm_arguments);
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
            .join(
                version
                    .assets
                    .as_ref()
                    .ok_or(Error::InvalidVersionJson("assets".to_string()))?,
            )
            .to_string_lossy()
            .to_string(),
    );
    game_options.insert(
        "asset_index",
        version
            .asset_index
            .ok_or(Error::InvalidVersionJson("assetIndex".to_string()))?
            .id,
    );
    game_options.insert(
        "assets_index_name",
        version
            .assets
            .ok_or(Error::InvalidVersionJson("assets".to_string()))?,
    );
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
            .game_arguments
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
    Ok(command_arguments)
}

/// Resolves the classpath string needed for the Java launch command.
///
/// This includes library paths (unzipping native libraries if needed),
/// extra classpaths, and the version jar or inheritance jars.
///
/// # Arguments
///
/// * `version` - The resolved Minecraft version metadata.
/// * `minecraft` - Reference to MinecraftLocation for path resolving.
/// * `extra_class_paths` - Additional class paths as a string.
///
/// # Returns
///
/// A string with the complete classpath, joined by platform-specific delimiter.
fn resolve_classpath(
    version: &ResolvedVersion,
    minecraft: &MinecraftLocation,
    extra_class_paths: String,
) -> String {
    let mut classpath = version
        .libraries
        .iter()
        .filter_map(|lib| match lib {
            ResolvedLibrary::Native(native_library) => {
                let path = minecraft.get_library_by_path(&native_library.path);
                let native_folder = minecraft.get_natives_root(&version.id);
                info!("Unzip native library {path:#?} to {native_folder:#?}");
                if let Ok(file) = std::fs::File::open(path)
                    && let Ok(mut zip_archive) = ZipArchive::new(file)
                {
                    decompression_all(&mut zip_archive, &native_folder).unwrap_or(());
                }
                None
            }
            ResolvedLibrary::Common(common_library) => Some(
                minecraft
                    .get_library_by_path(common_library.path.clone())
                    .to_string_lossy()
                    .to_string(),
            ),
        })
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
    let regex = Regex::new(r"\$\{(.*?)}").expect("Internal Error");

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

fn decompression_all<R: Read + io::Seek, S: AsRef<OsStr> + ?Sized>(
    zip_archive: &mut ZipArchive<R>,
    to: &S,
) -> Result<()> {
    let to = Path::new(to).to_path_buf();
    for i in 0..zip_archive.len() {
        let mut zip_file = zip_archive.by_index(i)?;
        let name = zip_file.name().to_string();
        let path = to.join(&name);
        let mut entry_content = vec![];
        zip_file.read_to_end(&mut entry_content)?;
        if zip_file.is_dir() {
            std::fs::create_dir_all(zip_file.name())?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, entry_content)?;
    }
    Ok(())
}
