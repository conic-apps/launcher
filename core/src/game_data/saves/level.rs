// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;

use anyhow::{anyhow, Result};
use nbt::{Blob, Value};
use serde::{Deserialize, Serialize};

use super::gamerule::GameRules;
use crate::utils::nbt::modify_nbt;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Level {
    #[serde(rename = "Data")]
    pub data: LevelData,
}

// todo: 获取1.15之前的世界生成器设置
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct LevelData {
    /// 1 or 0 (true/false) - true if cheats are enabled.
    #[serde(rename = "allowCommands")]
    pub allow_commands: bool,

    /// Center of the world border on the X coordinate. Defaults to 0.
    #[serde(rename = "BorderCenterX")]
    pub border_center_x: f64,

    /// Center of the world border on the Z coordinate. Defaults to 0.
    #[serde(rename = "BorderCenterZ")]
    pub border_center_z: f64,

    /// Defaults to 0.2.
    #[serde(rename = "BorderDamagePerBlock")]
    pub border_damage_per_block: f64,

    /// Width and length of the border of the border. Defaults to 60000000.
    #[serde(rename = "BorderSize")]
    pub border_size: f64,

    /// Defaults to 5.
    #[serde(rename = "BorderSafeZone")]
    pub border_safe_zone: f64,

    /// Defaults to 60000000.
    #[serde(rename = "BorderSizeLerpTarget")]
    pub border_size_lerp_target: f64,

    /// Defaults to 0.
    #[serde(rename = "BorderSizeLerpTime")]
    pub border_size_lerp_time: f64,

    /// Defaults to 5.
    #[serde(rename = "BorderWarningBlocks")]
    pub border_warning_blocks: f64,

    /// Defaults to 15.
    #[serde(rename = "BorderWarningTime")]
    pub border_warning_time: f64,

    /// The number of ticks until "clear weather" has ended.
    #[serde(rename = "clearWeatherTime")]
    pub clear_weather_time: i32,

    /// A collection of bossbars.
    #[serde(rename = "CustomBossEvents")]
    pub custom_boss_events: HashMap<String, CustomBossEventItem>,

    /// Options for datapacks.
    #[serde(rename = "DataPacks")]
    pub data_packs: DataPacksOptions,

    /// An integer displaying the data version.
    #[serde(rename = "DataVersion")]
    pub data_version: i32,

    /// The time of day. 0 is sunrise, 6000 is mid day, 12000 is sunset, 18000 is mid night, 24000
    /// is the next day's 0. This value keeps counting past 24000 and does not reset to 0.
    #[serde(rename = "DayTime")]
    pub day_time: i64,

    /// The current difficulty setting. 0 is Peaceful, 1 is Easy, 2 is Normal, and 3 is Hard.
    /// Defaults to 2.
    #[serde(rename = "Difficulty")]
    pub difficulty: u8,

    /// 1 or 0 (true/false) - True if the difficulty has been locked. Defaults to 0.
    #[serde(rename = "DifficultyLocked")]
    pub difficulty_locked: bool,

    /// Data for the ender dragon fight. Only appears after the end is entered.
    #[serde(rename = "DragonFight")]
    pub dragon_flight: DragonFlight,

    // pub enabled_features: Option<Vec<String>>,
    /// The gamerules used in the world.
    #[serde(rename = "GameRules")]
    pub game_rules: GameRules,

    /// The default game mode for the singleplayer player when they initially spawn. 0 is Survival,
    /// 1 is Creative, 2 is Adventure, 3 is Spectator.
    ///
    /// Note: Singleplayer worlds do not use this field to save which game mode the player is
    /// currently in.
    #[serde(rename = "GameType")]
    pub game_type: u8,

    /// 1 or 0 (true/false) - true if the player will respawn in Spectator on death in singleplayer.
    /// Affects all three game modes.
    pub hardcore: bool,

    /// 1 or 0 (true/false) - Normally true after a world has been initialized properly after
    /// creation. If the initial simulation was canceled somehow, this can be false and the world
    /// will be re-initialized on next load.
    pub initialized: bool,

    /// The Unix time in milliseconds when the level was last loaded.
    #[serde(rename = "LastPlayed")]
    pub last_played: i64,

    /// The name of the level.
    #[serde(rename = "LevelName")]
    pub level_name: String,

    // todo: pub player: PlayerData,
    /// 1 or 0 (true/false) - true if the level is currently experiencing rain, snow, and cloud
    /// cover.
    pub raining: bool,

    /// The number of ticks before "raining" is toggled and this value gets set to another random
    /// value.
    #[serde(rename = "rainTime")]
    pub rain_time: i32,

    /// A list of scheduled events
    #[serde(rename = "ScheduledEvents")]
    pub scheduled_events: Vec<i32>,

    /// Open the nameplate list of servers that have opened this archive
    #[serde(rename = "ServerBrands")]
    pub server_brands: Vec<String>,

    /// Birth angle
    #[serde(rename = "SpawnAngle")]
    pub spawn_angle: f64,

    /// The X coordinate of the world spawn.
    #[serde(rename = "SpawnX")]
    pub spawn_x: i32,

    /// The Y coordinate of the world spawn.
    #[serde(rename = "SpawnY")]
    pub spawn_y: i32,

    /// The Z coordinate of the world spawn.
    #[serde(rename = "SpawnZ")]
    pub spawn_z: i32,

    /// 1 or 0 (true/false) - If "raining" is true : true if the rain/snow/cloud cover is a lightning
    /// storm and dark enough for mobs to spawn under the sky. If "raining" is false, this has no
    /// effect.
    pub thundering: bool,

    /// The number of ticks before "thundering" is toggled and this value gets set to another random
    /// value.
    #[serde(rename = "thunderTime")]
    pub thunder_time: i32,

    /// The number of ticks since the start of the level.
    #[serde(rename = "Time")]
    pub time: i64,

    /// The NBT version of the level, with 1.14.4 being 19133.
    pub version: i32,

    /// Information about the Minecraft version the world was saved in.
    #[serde(rename = "Version")]
    pub version_info: WorldVersion,

    /// The UUID of the current wandering trader in the world saved as four ints.
    #[serde(rename = "WanderingTraderId")]
    pub wandering_trader_id: Option<Vec<i32>>,

    /// The current chance of the wandering trader spawning next attempt; this value is the
    /// percentage and will be divided by 10 when loaded by the game, for example a value of 50
    /// means 5.0% chance.
    #[serde(rename = "WanderingTraderSpawnChance")]
    pub wandering_trader_spawn_chance: i32,

    /// The amount of ticks until another wandering trader is attempted to spawn
    #[serde(rename = "WanderingTraderSpawnDelay")]
    pub wandering_trader_spawn_delay: i32,

    /// 1 or 0 (true/false) - true if the world was opened in a modified version.
    #[serde(rename = "WasModded")]
    pub was_modded: bool,

    /// World and dimension generation settings
    #[serde(rename = "WorldGenSettings")]
    pub world_gen_settings: WorldGenSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomBossEventItem {
    pub color: String,
    pub create_world_fog: bool,
    pub darken_screen: bool,
    pub max: i32,
    pub name: String,
    pub overlay: String,
    pub play_boss_music: bool,
    pub players: Vec<Vec<i32>>,
    pub value: i32,
    pub visible: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DataPacksOptions {
    pub disabled: Vec<String>,
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DragonFlight {
    pub dragon: Option<Vec<i32>>,
    pub dragon_killed: bool,
    pub exit_portal_location: Option<ExitPortalLocation>,
    pub gateways: Vec<i32>,
    pub needs_state_scanning: bool,
    pub previously_killed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExitPortalLocation {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A scheduled event
pub struct ScheduledEvent {
    /// Event callbacks
    pub callback: ScheduledEventCallback,

    /// Event name
    pub name: String,

    /// Trigger time
    pub trigger_time: i64,
}

/// Event callbacks
pub struct ScheduledEventCallback {
    /// The callback name, which is a namespace ID
    pub name: String,
    /// The callback type. Can be minecraft:function (function) and minecraft:function_tag (function
    /// tag).
    pub r#type: String,
}

///  Information about the Minecraft version the world was saved in.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorldVersion {
    /// An integer displaying the data version.
    pub id: i32,

    ///  The version name as a string, e.g. "15w32b".
    pub name: String,

    /// Developing series. In 1.18 experimental snapshots, it was set to "ccpreview". In others, set
    /// to "main".
    pub series: String,

    /// 1 or 0 (true/false) – Whether the version is a snapshot or not.
    pub snapshot: bool,
}

/// World and dimension generation settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorldGenSettings {
    /// Indicates whether a reward box is generated near the spawn point when the player first enters
    /// the game, which is only valid for single-player play.
    pub bouns_chest: Option<bool>,

    /// Contains all dimensions of the world
    pub dimensions: HashMap<String, Blob>,

    /// Indicates whether the world generates a structure
    pub generate_features: bool,

    /// Seeds of the world in digital form
    pub seed: i64,
}

/// Get level data
///
/// Note: This function will return the `Data` tag in `level.dat`
pub fn get_level_data<P: AsRef<Path>>(level_path: P) -> Result<LevelData> {
    let file = fs::File::open(level_path)?;
    Ok(nbt::from_gzip_reader::<_, Level>(file)?.data)
}

/// Modify level
///
/// * `value_path` - You need to use a colon to connect the path. For example, if you want to modify the
///   seed, you should use `world_gen_settings:seed` or `Data:world_gen_settings:seed`.
pub fn modify_level<P: AsRef<Path>>(world_path: P, value_path: &str, value: Value) -> Result<()> {
    let level_path = world_path.as_ref().to_path_buf().join("level.dat");

    let mut file = fs::File::options().read(true).open(&level_path)?;
    let buffer = BufReader::new(&mut file);

    let level: Blob = nbt::from_gzip_reader(buffer)?;
    let level_data = level
        .get("Data")
        .ok_or(anyhow!("level.dat file is broken"))?
        .clone();
    let level_data = modify_nbt(level_data, value_path, value)?;
    let mut result = Blob::new();
    result.insert("Data", level_data)?;
    drop(file);

    let mut file = fs::File::options().write(true).open(level_path)?;
    result.to_gzip_writer(&mut file)?;

    Ok(())
}

/// Get all levels from 'saves' folder
pub fn get_all_levels<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Level>> {
    let mut levels = HashMap::new();

    for dir_entry in fs::read_dir(path)? {
        let dir_entry = match dir_entry {
            Err(_) => continue,
            Ok(dir_entry) => dir_entry,
        };
        match dir_entry.metadata() {
            Err(_) => continue,
            Ok(metadata) => {
                if !metadata.is_dir() {
                    continue;
                }
            }
        }

        let file = dir_entry.path().join("level.dat");
        let name = dir_entry.file_name().to_string_lossy().to_string();
        let file = fs::File::open(file);
        if file.is_err() {
            continue;
        }
        let file = file.unwrap();
        let level = match nbt::from_gzip_reader(file) {
            Ok(level) => level,
            Err(_) => continue,
        };
        levels.insert(name, level);
    }

    Ok(levels)
}
