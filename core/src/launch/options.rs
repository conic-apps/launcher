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

use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::Value;

use crate::core::{folder::MinecraftLocation, version::Version};

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone)]
pub enum UserType {
    Mojang,
    Legacy,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub ip: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct YggdrasilAgent {
    /// The jar file path of the authlib-injector
    pub jar: PathBuf,

    /// The auth server host
    pub server: String,

    /// The prefetched base64
    pub prefetched: Option<String>,
}

/// Game process priority, invalid on windows
#[derive(Debug, Clone)]
pub enum ProcessPriority {
    High,
    AboveNormal,
    Normal,
    BelowNormal,
    Low,
}

/// User custom jvm gc
#[derive(Debug, Clone)]
pub enum GC {
    Serial,
    Parallel,
    ParallelOld,
    G1,
    Z,
}

#[derive(Debug, Clone)]
/// Launch options for game
pub struct LaunchOptions {
    /// User selected game profile.
    ///
    /// For game display name & uuid
    pub(crate) game_profile: GameProfile,

    pub(crate) access_token: String,
    pub(crate) user_type: UserType,
    pub(crate) properties: String,
    pub(crate) launcher_name: String,
    pub(crate) launcher_version: String,

    /// Overwrite the version name of the current version.
    ///
    /// If this is absent, it will use version name from resolved version.
    pub(crate) version_name: Option<String>,

    /// Overwrite the version type of the current version.
    ///
    /// If this is absent, it will use version type from resolved version.
    ///  
    /// Some people use this to show fantastic message on the welcome screen.
    pub(crate) version_type: Option<String>,

    /// The full path of launched game icon
    ///
    /// Currently, this only supported on MacOS
    pub(crate) game_icon: Option<PathBuf>,

    /// The launched game name
    ///
    /// Currently, this only supported on MacOS.
    pub(crate) game_name: String,

    /// The path of parent directory of `saves` / `logs` / `configs` / `mods` / `resourcepacks`.
    ///
    /// If None, will be generated using the version_id passed in at startup.
    ///
    /// ### WARN: If it is not an absolute path, the game will not save game data in the correct location.
    pub(crate) game_path: PathBuf,

    /// The path of parent directory of `assets` / `libraries`, like `.minecraft` folder.
    pub(crate) resource_path: PathBuf,

    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: u32,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: u32,

    /// Directly launch to a server.
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

    // Todo: yggdrasilAgent
    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: bool,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: bool,

    /// Add extra classpath
    pub(crate) extra_class_paths: Option<Vec<String>>,

    /// The path of parent directory of `<version_id>.jar` and `<version_id>.json`,
    ///
    /// default is `versions/{version_id}`
    ///
    /// ### WARN: If you have not saved `version.jar` and `version.json` to the default location, please modify this after creating the Launcher, otherwise related operations will return Err()
    pub(crate) version_root: PathBuf,

    /// The version of launched Minecraft. Can be either resolved version or version string
    pub(crate) version: Version,

    /// Enable features. Not really in used...
    pub(crate) features: HashMap<String, Value>,

    /// Game process priority, invalid on windows
    pub(crate) process_priority: ProcessPriority,

    /// Support yushi's yggdrasil agent <https://github.com/to2mbn/authlib-injector/wiki>
    pub(crate) yggdrasil_agent: Option<YggdrasilAgent>,

    pub(crate) version_id: String,

    pub(crate) gc: GC,

    pub(crate) minecraft_location: MinecraftLocation,

    pub(crate) native_path: PathBuf,
}

impl LaunchOptions {
    /// spawn an instance with default launch options
    pub async fn new(version_id: &str, minecraft: &MinecraftLocation) -> Result<Self> {
        let version_json_path = minecraft.get_version_json(version_id);
        let raw_version_json = tokio::fs::read_to_string(version_json_path).await?;
        let version_json: Version = serde_json::from_str((&raw_version_json).as_ref())?;

        Ok(Self {
            game_profile: GameProfile {
                name: "Steve".to_string(),
                uuid: uuid::Uuid::new_v4().to_string().replace('-', ""),
            },
            access_token: uuid::Uuid::new_v4().to_string().replace('-', ""),
            user_type: UserType::Mojang,
            properties: "{}".to_string(),
            launcher_name: "AmethystLauncher".to_string(),
            launcher_version: "0.0.1".to_string(),
            version_name: None,
            version_type: None,
            game_icon: None,
            game_name: "Minecraft".to_string(),
            game_path: minecraft.get_version_root(version_id),
            version_root: minecraft.get_version_root(version_id),
            resource_path: minecraft.root.clone(),
            min_memory: 128,
            max_memory: 2048,
            server: None,
            width: 854,
            height: 480,
            fullscreen: false,
            extra_jvm_args: vec![],
            extra_mc_args: vec![],
            is_demo: false,
            ignore_invalid_minecraft_certificates: false,
            ignore_patch_discrepancies: false,
            extra_class_paths: None,
            version: version_json,
            features: HashMap::new(),
            yggdrasil_agent: None,
            process_priority: ProcessPriority::Normal,
            version_id: version_id.to_string(),
            gc: GC::G1,
            minecraft_location: minecraft.clone(),
            native_path: minecraft.get_natives_root(version_id),
        })
    }

    pub async fn new_forge_options(
        version_id: &str,
        minecraft: &MinecraftLocation,
    ) -> Result<Self> {
        let mut default = LaunchOptions::new(version_id, &minecraft).await?;

        let version = minecraft.get_version_json(version_id);
        let version = tokio::fs::read_to_string(version).await?;
        let version = serde_json::from_str::<Version>(&version)?;

        if let Some(arguments) = version.minecraft_arguments {
            let tweak_class_regex = Regex::new(r"--tweakClass\s+.+").unwrap();
            let forge_arguments = tweak_class_regex
                .captures(&arguments)
                .ok_or(anyhow!("tweakClass not found"))?
                .get(0)
                .ok_or(anyhow!("tweakClass not found"))?
                .as_str();
            default.extra_mc_args.extend(
                forge_arguments
                    .split(" ")
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>(),
            );
        }

        default.extra_jvm_args.extend(vec![
            "\"-Dfml.ignoreInvalidMinecraftCertificates=true\"".to_string(),
            "\"-Dfml.ignorePatchDiscrepancies=true\"".to_string(),
        ]);

        Ok(default)
    }
}
