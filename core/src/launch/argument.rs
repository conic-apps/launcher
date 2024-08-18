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



// use std::{
//     collections::{HashMap, HashSet},
//     env,
//     path::PathBuf,
// };

// use anyhow::Result;
// use regex::Regex;
// use tokio::{fs, process::Command};
// use zip::ZipArchive;

// use crate::{
//     utils::unzip::decompression_all,
//     {folder::MinecraftLocation, version::ResolvedVersion, OsType, PlatformInfo},
// };

// use super::options::{LaunchOptions, ProcessPriority, UserType, GC};

// /// launch arguments for launch
// ///
// /// You can use `from_launch_options` to generate launch parameters and use `to_launch_command` to
// /// convert to shell commands
// #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
// pub struct LaunchArguments(Vec<String>);

// const DEFAULT_GAME_ICON: &[u8] = include_bytes!("./assets/minecraft.icns");

// impl LaunchArguments {
//     pub async fn from_launch_options(
//         launch_options: LaunchOptions,
//         version: ResolvedVersion,
//         platform: &PlatformInfo,
//     ) -> Result<Self> {
//         let minecraft = MinecraftLocation::new(&launch_options.resource_path);

//         let game_icon = match launch_options.game_icon.clone() {
//             Some(icon_path) => icon_path,
//             None => {
//                 let icon_path = minecraft.assets.join("minecraft.icns");
//                 tokio::fs::write(&icon_path, DEFAULT_GAME_ICON).await?;
//                 icon_path
//             }
//         };

//         let mut command_arguments = Vec::new();

//         command_arguments.push(format!(
//             "-Dminecraft.client.jar={version_jar}",
//             version_jar = launch_options
//                 .version_root
//                 .join(&format!("{}.jar", &launch_options.version_id))
//                 .to_string_lossy()
//         ));

//         if platform.os_type == OsType::Osx {
//             command_arguments.push(format!(
//                 "-Xdock:name={game_name}",
//                 game_name = launch_options.game_name
//             ));
//             command_arguments.push(format!(
//                 "-Xdock:icon={game_icon}",
//                 game_icon = game_icon.to_string_lossy()
//             ));
//         }

//         command_arguments.push(format!("-Xms{}M", launch_options.min_memory));
//         command_arguments.push(format!("-Xmx{}M", launch_options.max_memory));

//         if launch_options.ignore_invalid_minecraft_certificates {
//             command_arguments.push("-Dfml.ignoreInvalidMinecraftCertificates=true".to_string());
//         }
//         if launch_options.ignore_patch_discrepancies {
//             command_arguments.push("-Dfml.ignorePatchDiscrepancies=true".to_string());
//         }

//         match launch_options.gc {
//             GC::G1 => {
//                 command_arguments.extend([
//                     "-XX:+UseG1GC".to_string(),
//                     "-XX:+UnlockExperimentalVMOptions".to_string(),
//                     "-XX:G1NewSizePercent=20".to_string(),
//                     "-XX:G1ReservePercent=20".to_string(),
//                     "-XX:MaxGCPauseMillis=50".to_string(),
//                     "-XX:G1HeapRegionSize=16M".to_string(),
//                 ]);
//             }
//             GC::Parallel => {
//                 command_arguments.extend([
//                     "-XX:+UseParallelGC".to_string(),
//                     format!(
//                         "-XX:ParallelGCThreads={num}",
//                         num = num_cpus::get_physical()
//                     ),
//                 ]);
//             }
//             GC::ParallelOld => {
//                 command_arguments.push("-XX:+UseParallelOldGC".to_string());
//             }
//             GC::Serial => {
//                 command_arguments.push("-XX:+UseSerialGC".to_string());
//             }
//             GC::Z => {
//                 command_arguments.push("-XX:+UseZGC".to_string());
//             }
//         }

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

//         let mut jvm_options: HashMap<&str, String> = HashMap::new();
//         jvm_options.insert(
//             "natives_directory",
//             launch_options.native_path.to_string_lossy().to_string(),
//         );
//         jvm_options.insert("launcher_name", launch_options.launcher_name.clone());
//         jvm_options.insert("launcher_version", launch_options.launcher_version.clone());
//         jvm_options.insert(
//             "classpath",
//             resolve_classpath(
//                 &launch_options,
//                 &version,
//                 &minecraft,
//                 launch_options.extra_class_paths.clone(),
//             ),
//         );

//         let mut jvm_arguments = version.arguments.jvm;
//         if let Some(client) = version.logging.get("client") {
//             let argument = &client.argument;
//             let file_path = minecraft.get_version_root(&version.id).join("log4j2.xml");
//             if tokio::fs::try_exists(&file_path).await? {
//                 jvm_arguments.push(format!(
//                     "\"{}\"",
//                     argument.replace("${path}", &file_path.to_string_lossy().to_string())
//                 ));
//             }
//         }

//         command_arguments.extend(
//             jvm_arguments
//                 .iter()
//                 .map(|arg| format(arg, jvm_options.clone())),
//         );
//         command_arguments.extend(launch_options.extra_jvm_args);

//         command_arguments.push(
//             version
//                 .main_class
//                 .unwrap_or("net.minecraft.client.main.Main".to_string()),
//         );

//         let mut game_options = HashMap::with_capacity(13);

//         let assets_dir = launch_options.resource_path.join("assets");
//         game_options.insert(
//             "version_name",
//             match launch_options.version_name {
//                 Some(v) => v,
//                 None => version.id,
//             },
//         );
//         game_options.insert(
//             "version_type",
//             match launch_options.version_type {
//                 Some(v) => v,
//                 None => version.version_type,
//             },
//         );
//         game_options.insert("assets_root", assets_dir.to_string_lossy().to_string());
//         game_options.insert(
//             "game_assets",
//             assets_dir
//                 .join("virtual")
//                 .join(&version.assets)
//                 .to_string_lossy()
//                 .to_string(),
//         );
//         game_options.insert(
//             "asset_index",
//             version
//                 .asset_index
//                 .ok_or(anyhow::anyhow!(
//                     "asset index is not fount! version.json is broken"
//                 ))?
//                 .id,
//         );
//         game_options.insert("assets_index_name", version.assets);
//         game_options.insert(
//             "game_directory",
//             launch_options.game_path.to_string_lossy().to_string(),
//         );
//         game_options.insert("auth_player_name", launch_options.game_profile.name);
//         game_options.insert("auth_uuid", launch_options.game_profile.uuid);
//         game_options.insert("auth_access_token", launch_options.access_token);
//         game_options.insert("user_properties", launch_options.properties);
//         game_options.insert(
//             "user_type",
//             match launch_options.user_type {
//                 UserType::Mojang => "mojang".to_string(),
//                 UserType::Legacy => "legacy".to_string(),
//             },
//         );
//         game_options.insert("resolution_width", launch_options.width.to_string());
//         game_options.insert("resolution_height", launch_options.height.to_string());

//         command_arguments.extend(
//             version
//                 .arguments
//                 .game
//                 .iter()
//                 .map(|arg| format(arg, game_options.clone())),
//         );
//         command_arguments.extend(launch_options.extra_mc_args);
//         if let Some(server) = launch_options.server {
//             command_arguments.extend(vec!["--server".to_string(), server.ip]);
//             if let Some(port) = server.port {
//                 command_arguments.extend(vec!["--port".to_string(), port.to_string()])
//             }
//         }
//         if launch_options.fullscreen {
//             command_arguments.push("--fullscreen".to_string());
//         }
//         let no_width_arguments = None
//             == command_arguments
//                 .iter()
//                 .find(|v| v == &&"--width".to_string());
//         if no_width_arguments && !launch_options.fullscreen {
//             command_arguments.extend(vec![
//                 "--width".to_string(),
//                 launch_options.width.to_string(),
//                 "--height".to_string(),
//                 launch_options.height.to_string(),
//             ]);
//         }

//         Ok(LaunchArguments(command_arguments))
//     }

//     /// spawn a command instance, you can use this to launch the game
//     pub async fn to_async_command(
//         &self,
//         launch_options: LaunchOptions,
//         platform: &PlatformInfo,
//     ) -> Result<std::process::Command> {
//         let mut command = format!(
//             "cd {}\n",
//             launch_options.version_root.to_string_lossy().to_string()
//         );
//         match platform.os_type {
//             OsType::Windows => {}
//             _ => command.push_str("nice "),
//         };

//         if let OsType::Windows = platform.os_type {
//             command.push_str(" -c ");
//         }

//         if platform.os_type != OsType::Windows {
//             match launch_options.process_priority {
//                 ProcessPriority::High => {
//                     command.push_str("-n 0 ");
//                 }
//                 ProcessPriority::AboveNormal => {
//                     command.push_str("-n 5 ");
//                 }
//                 ProcessPriority::Normal => (), // nothing to do
//                 ProcessPriority::BelowNormal => {
//                     command.push_str("-n 15 ");
//                 }
//                 ProcessPriority::Low => {
//                     command.push_str("-n 19 ");
//                 }
//             };
//         }
//         // todo(after java exec): add -Dfile.encoding=encoding.name() and other
//         let mut launch_command = java_exec.binary.to_string_lossy().to_string();
//         launch_command.push_str(" ");
//         launch_command.push_str(&self.0.clone().join(" "));
//         command.push_str(&launch_command);
//         match platform.os_type {
//             OsType::Windows => command.push_str(&format!(
//                 "\ndel /F /Q {}\n",
//                 launch_options.native_path.to_string_lossy()
//             )),
//             _ => command.push_str(&format!(
//                 "\n rm -rf {}",
//                 launch_options.native_path.to_string_lossy()
//             )),
//         }
//         let script_path = match platform.os_type {
//             OsType::Linux => launch_options.version_root.join(".cache").join("launch.sh"),
//             OsType::Osx => launch_options.version_root.join(".cache").join("launch.sh"),
//             OsType::Windows => launch_options
//                 .version_root
//                 .join(".cache")
//                 .join("launch.bat"),
//         };

//         fs::create_dir_all(script_path.parent().unwrap()).await?;
//         fs::write(&script_path, command).await?;

//         let mut command = match platform.os_type {
//             OsType::Windows => {
//                 let vars = env::vars().find(|v| v.0 == "PATH").unwrap();

//                 let path_vars = vars.1.as_str().split(";").collect::<Vec<&str>>(); // todo: test it in windows
//                 let powershell_folder = PathBuf::from(
//                     path_vars
//                         .into_iter()
//                         .find(|v| v.to_lowercase().contains("powershell"))
//                         .unwrap(),
//                 );

//                 std::process::Command::new(
//                     powershell_folder
//                         .join("powershell.exe")
//                         .to_string_lossy()
//                         .to_string(),
//                 )
//             }
//             _ => {
//                 let mut chmod = Command::new("chmod");
//                 chmod.args(&["+x", script_path.to_string_lossy().to_string().as_ref()]);
//                 chmod.status().await?;
//                 std::process::Command::new("bash")
//             }
//         };
//         command.arg(script_path);
//         Ok(command)
//     }
// }

// fn resolve_classpath(
//     options: &LaunchOptions,
//     version: &ResolvedVersion,
//     minecraft: &MinecraftLocation,
//     extra_class_paths: Option<Vec<String>>,
// ) -> String {
//     let mut classpath = version
//         .libraries
//         .iter()
//         .filter(|lib| {
//             if lib.is_native_library {
//                 let path = minecraft.get_library_by_path(&lib.download_info.path);
//                 let native_folder = options.native_path.clone();
//                 println!("{:#?},{:#?}", path, native_folder);
//                 if let Ok(file) = std::fs::File::open(path) {
//                     if let Ok(mut zip_archive) = ZipArchive::new(file) {
//                         decompression_all(&mut zip_archive, &native_folder).unwrap_or(());
//                     }
//                 }
//             }
//             // true
//             !lib.is_native_library
//         })
//         .map(|lib| {
//             minecraft
//                 .get_library_by_path(lib.download_info.path.clone())
//                 .to_string_lossy()
//                 .to_string()
//         })
//         .collect::<HashSet<String>>()
//         .into_iter()
//         .collect::<Vec<String>>();

//     classpath.push(
//         minecraft
//             .get_version_jar(version.id.clone(), None)
//             .to_str()
//             .unwrap()
//             .to_string(),
//     );

//     if let Some(extra_class_paths) = extra_class_paths {
//         classpath.extend(extra_class_paths);
//     }
//     classpath.join(DELIMITER)
// }

// fn format(template: &str, args: HashMap<&str, String>) -> String {
//     let regex = Regex::new(r"\$\{(.*?)}").unwrap();

//     regex
//         .replace_all(&template, |caps: &regex::Captures| {
//             let key = String::from(&caps[1]);
//             let value = args.get(&caps[1]).unwrap_or(&key);
//             value.to_string()
//         })
//         .to_string()
// }
