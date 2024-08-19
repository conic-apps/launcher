/*
 * Amethyst Launcher Core
 * Copyright (C) 2023 Broken-Deer <old_driver__@outlook.com> and contributors
 *
 * This program is free software, you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    collections::{HashMap, HashSet},
    io::BufRead,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

use log::{debug, error, info};
use regex::Regex;
use zip::ZipArchive;

use crate::{
    config::{
        instance::InstanceConfig,
        launch::{LaunchConfig, ProcessPriority, Server, GC},
    },
    folder::MinecraftLocation,
    platform::{OsType, DELIMITER},
    utils::unzip::decompression_all,
    version::{ResolvedVersion, Version},
    APP_VERSION, DATA_LOCATION, PLATFORM_INFO,
};

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// User selected game profile.
    ///
    /// For game display name & uuid
    pub(crate) game_profile: GameProfile,

    pub(crate) properties: String,
    pub(crate) access_token: String,

    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: u32,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: u32,

    /// Directly launch to a server. TODO: support 1.21.1
    pub(crate) server: Option<Server>,

    /// window width
    pub(crate) width: u32,

    /// window height
    pub(crate) height: u32,

    pub(crate) fullscreen: bool,

    /// User custom additional java virtual machine command line arguments.
    ///
    /// If this is empty, the `DEFAULT_EXTRA_JVM_ARGS` will be used.
    pub(crate) extra_jvm_args: Vec<String>,

    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: Vec<String>,

    pub(crate) is_demo: bool,

    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: bool,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: bool,

    /// Add extra classpath
    pub(crate) extra_class_paths: Vec<String>,

    /// Game process priority, invalid on windows
    pub(crate) process_priority: ProcessPriority,

    // /// TODO: Support yushi's yggdrasil agent <https://github.com/to2mbn/authlib-injector/wiki>
    // pub(crate) yggdrasil_agent: Option<YggdrasilAgent>,
    pub(crate) gc: GC,

    pub(crate) minecraft_location: MinecraftLocation,
}

impl LaunchOptions {
    pub fn get(instance_config: InstanceConfig) -> Self {
        let global_config = LaunchConfig::get();
        let instance_config = instance_config.launch_config.clone();
        Self {
            game_profile: GameProfile {
                name: "Steve".to_string(),
                uuid: "00000000-0000-0000-0000-000000011111".to_string(),
            },
            access_token: "00000000000000000000000000000000".to_string(),
            min_memory: instance_config
                .min_memory
                .unwrap_or(global_config.min_memory),
            max_memory: instance_config
                .max_memory
                .unwrap_or(global_config.max_memory),
            server: instance_config.server,
            width: instance_config.width.unwrap_or(global_config.width),
            height: instance_config.height.unwrap_or(global_config.height),
            fullscreen: instance_config
                .fullscreen
                .unwrap_or(global_config.fullscreen),
            extra_jvm_args: instance_config
                .extra_jvm_args
                .unwrap_or(global_config.extra_jvm_args),
            extra_mc_args: instance_config
                .extra_mc_args
                .unwrap_or(global_config.extra_mc_args),
            is_demo: instance_config.is_demo.unwrap_or(global_config.is_demo),
            ignore_invalid_minecraft_certificates: instance_config
                .ignore_invalid_minecraft_certificates
                .unwrap_or(global_config.ignore_invalid_minecraft_certificates),
            ignore_patch_discrepancies: instance_config
                .ignore_patch_discrepancies
                .unwrap_or(global_config.ignore_patch_discrepancies),
            extra_class_paths: instance_config
                .extra_class_paths
                .unwrap_or(global_config.extra_class_paths),
            process_priority: instance_config
                .process_priority
                .unwrap_or(global_config.process_priority),
            gc: instance_config.gc.unwrap_or(global_config.gc),
            minecraft_location: MinecraftLocation::new(&DATA_LOCATION.get().unwrap().root),
            properties: "{}".to_string(),
        }
    }
}

static DEFAULT_GAME_ICON: &[u8] = include_bytes!("./assets/minecraft.icns");

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
    info!("Generating startup parameters");
    let game_icon = minecraft_location.assets.join("minecraft.icns");
    let version =
        Version::from_versions_folder(minecraft_location.clone(), &instance.get_version_id())
            .unwrap()
            .parse(&minecraft_location, platform)
            .await
            .unwrap();
    tokio::fs::write(&game_icon, DEFAULT_GAME_ICON)
        .await
        .unwrap();

    let mut command_arguments = Vec::new();

    command_arguments.push(format!(
        "\"-Dminecraft.client.jar={version_jar}\"",
        version_jar = minecraft_location
            .get_version_jar(&instance.runtime.minecraft, None)
            .to_string_lossy()
    ));
    if platform.os_type == OsType::Osx {
        command_arguments.push("-Xdock:name=Minecraft".to_string());
        command_arguments.push(format!(
            "-Xdock:icon={game_icon}",
            game_icon = game_icon.to_string_lossy()
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
    jvm_options.insert("launcher_name", "Amethyst Launcher".to_string());
    jvm_options.insert("launcher_version", APP_VERSION.get().unwrap().to_string());
    jvm_options.insert(
        "classpath",
        resolve_classpath(
            &version,
            &minecraft_location,
            launch_options.extra_class_paths.clone(),
        ),
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
                argument.replace("${path}", &file_path.to_string_lossy().to_string())
            ));
        }
    }
    jvm_arguments.extend(version.arguments.jvm);
    command_arguments.extend(launch_options.extra_jvm_args);
    command_arguments.extend(
        jvm_arguments
            .iter()
            .map(|arg| format(arg, jvm_options.clone())),
    );

    command_arguments.push(
        version
            .main_class
            .unwrap_or("net.minecraft.client.main.Main".to_string()),
    );

    let mut game_options: HashMap<&str, String> = HashMap::with_capacity(13);
    let assets_dir = minecraft_location.assets.clone();
    game_options.insert("version_name", version.id.clone());
    game_options.insert("version_type", version.version_type.unwrap());
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
        instance.get_instance_root().to_string_lossy().to_string(),
    );
    game_options.insert("auth_player_name", launch_options.game_profile.name);
    game_options.insert("auth_uuid", launch_options.game_profile.uuid);
    game_options.insert("auth_access_token", launch_options.access_token);
    game_options.insert("user_properties", launch_options.properties);
    game_options.insert("user_type", "mojang".to_string());
    game_options.insert("resolution_width", launch_options.width.to_string());
    game_options.insert("resolution_height", launch_options.height.to_string());

    command_arguments.extend(
        version
            .arguments
            .game
            .iter()
            .map(|arg| format(arg, game_options.clone())),
    );
    command_arguments.extend(launch_options.extra_mc_args);
    if let Some(server) = launch_options.server {
        command_arguments.extend(vec!["--server".to_string(), server.ip]);
        if let Some(port) = server.port {
            command_arguments.extend(vec!["--port".to_string(), port.to_string()])
        }
    }
    if launch_options.fullscreen {
        command_arguments.push("--fullscreen".to_string());
    }
    let no_width_arguments = None
        == command_arguments
            .iter()
            .find(|v| v == &&"--width".to_string());
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
    let mut command = format!("cd {}\n", version_root.to_string_lossy().to_string());
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
            chmod.args(&["+x", script_path.to_string_lossy().to_string().as_ref()]);
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
        error!("Minecraft exits with error code {}", output.status);
    } else {
        info!("Minecraft exits with error code {}", output.status);
    }
}

fn resolve_classpath(
    version: &ResolvedVersion,
    minecraft: &MinecraftLocation,
    extra_class_paths: Vec<String>,
) -> String {
    let mut classpath = version
        .libraries
        .iter()
        .filter(|lib| {
            if lib.is_native_library {
                let path = minecraft.get_library_by_path(&lib.download_info.path);
                let native_folder = minecraft.get_natives_root(&version.id);
                println!("{:#?},{:#?}", path, native_folder);
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

    classpath.push(
        minecraft
            .get_version_jar(version.id.clone(), None)
            .to_str()
            .unwrap()
            .to_string(),
    );

    if !extra_class_paths.is_empty() {
        classpath.extend(extra_class_paths);
    }
    classpath.join(DELIMITER)
}

fn format(template: &str, args: HashMap<&str, String>) -> String {
    let regex = Regex::new(r"\$\{(.*?)}").unwrap();

    regex
        .replace_all(&template, |caps: &regex::Captures| {
            let key = String::from(&caps[1]);
            let value = args.get(&caps[1]).unwrap_or(&key);
            value.to_string()
        })
        .to_string()
}
