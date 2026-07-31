// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CRUD implementation for game instance

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use flate2::read::GzDecoder;
use folder::DATA_LOCATION;
use futures::TryStreamExt;
use log::{debug, info};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Runtime, command};
use uuid::Uuid;

mod config;
mod error;

pub use config::*;
pub use error::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("instance")
        .invoke_handler(tauri::generate_handler![
            cmd_create_instance,
            cmd_list_instances,
            cmd_get_instance_by_id,
            cmd_update_instance,
            cmd_delete_instance,
            cmd_add_background_file,
            cmd_get_background_path,
            cmd_calculate_playtime
        ])
        .build()
}

#[command]
async fn cmd_create_instance(config: InstanceConfig, id: Option<Uuid>) -> Result<Uuid> {
    create_instance(config, id).await
}

#[command]
async fn cmd_list_instances(sort_by: SortBy) -> Result<Vec<Instance>> {
    list_instances(sort_by).await
}

#[command]
async fn cmd_get_instance_by_id(id: Uuid) -> Option<Instance> {
    get_instance_by_id(id).await
}

#[command]
async fn cmd_update_instance(config: InstanceConfig, id: Uuid) -> Result<()> {
    update_instance(config, id).await
}

#[command]
async fn cmd_delete_instance(id: Uuid) -> Result<()> {
    delete_instance(id).await
}

#[command]
async fn cmd_add_background_file(path: String, id: Uuid) -> Result<()> {
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    async_fs::copy(path, instance_root.join("background")).await?;
    Ok(())
}

#[command]
async fn cmd_get_background_path(id: Uuid) -> String {
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    instance_root
        .join("background")
        .to_string_lossy()
        .to_string()
}

#[command]
async fn cmd_calculate_playtime(id: Uuid) -> Result<u64> {
    calculate_playtime(id)
}

/// Creates a new game instance using the provided configuration.
pub async fn create_instance(config: InstanceConfig, id: Option<Uuid>) -> Result<Uuid> {
    let id = id.unwrap_or(Uuid::new_v4());
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    let config_file_path = instance_root.join("instance.toml");
    if let Some(parent) = config_file_path.parent() {
        async_fs::create_dir_all(parent).await?
    }
    async_fs::write(config_file_path, toml::to_string_pretty(&config)?).await?;
    info!("Created instance: {}", config.name);
    Ok(id)
}

/// Enum representing different sorting strategies for listing instances.
#[derive(Deserialize)]
pub enum SortBy {
    /// Sort by instance name.
    Name,
    // TODO: Other sort strategies, such as createdon, last played at, play frequency...
}

/// Reads all instances stored in the data directory
pub async fn list_instances(sort_by: SortBy) -> Result<Vec<Instance>> {
    let instances_folder = &DATA_LOCATION.instances;
    async_fs::create_dir_all(instances_folder).await?;
    let mut folder_entries = async_fs::read_dir(instances_folder).await?;
    let mut instances = Vec::new();

    while let Some(entry) = folder_entries.try_next().await? {
        let file_type = match entry.file_type().await {
            Err(_) => continue,
            Ok(file_type) => file_type,
        };
        if !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        let folder_name = match path.file_name() {
            None => continue,
            Some(x) => x,
        }
        .to_string_lossy()
        .to_string();
        debug!("Checking {folder_name}");
        let instance_config = path.join("instance.toml");
        let metadata = match instance_config.metadata() {
            Err(_) => continue,
            Ok(result) => result,
        };
        if metadata.len() > 2_000_000 || !instance_config.is_file() {
            continue;
        }
        let config_content = match async_fs::read_to_string(instance_config).await {
            Err(_) => continue,
            Ok(content) => content,
        };
        let instance = Instance {
            config: match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => config,
                Err(_) => continue,
            },
            installed: async_fs::metadata(path.join(".install.lock")).await.is_ok(),
            id: match Uuid::from_str(&folder_name) {
                Ok(x) => x,
                Err(_) => continue,
            },
        };
        instances.push(instance);
    }
    match sort_by {
        SortBy::Name => {
            instances.sort_by_key(|instance| instance.config.name.clone());
        }
    }
    Ok(instances)
}

pub async fn get_instance_by_id(id: Uuid) -> Option<Instance> {
    let instance_root = &DATA_LOCATION.get_instance_root(&id);
    let config_file = instance_root.join("instance.toml");
    if let Ok(config_file_content) = async_fs::read_to_string(config_file).await
        && let Ok(config) = toml::from_str::<InstanceConfig>(&config_file_content)
    {
        Some(Instance {
            config,
            installed: async_fs::metadata(instance_root.join(".install.lock"))
                .await
                .is_ok(),
            id,
        })
    } else {
        None
    }
}

/// Updates the configuration file of an existing instance
/// specified by the given UUID.
pub async fn update_instance(config: InstanceConfig, id: Uuid) -> Result<()> {
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    let config_file = instance_root.join("instance.toml");
    async_fs::write(config_file, toml::to_string_pretty(&config)?).await?;
    info!("Updated instance: {}", config.name);
    Ok(())
}

/// Deletes the instance directory corresponding to the given UUID.
pub async fn delete_instance(id: Uuid) -> Result<()> {
    async_fs::remove_dir_all(DATA_LOCATION.get_instance_root(&id)).await?;
    info!("Deleted {id}");
    Ok(())
}

/// Represents a game instance, including its configuration,
/// installation status, and unique ID.
#[derive(Deserialize, Serialize, Default)]
pub struct Instance {
    /// The configuration of the instance.
    pub config: InstanceConfig,
    /// Whether the instance has been installed.
    pub installed: bool,
    /// Unique identifier of the instance.
    pub id: Uuid,
}

impl Instance {
    pub fn get_version_id(&self) -> Result<String> {
        let config = &self.config;
        config
            .runtime
            .mod_loader_type
            .as_ref()
            .map(|mod_loader_type| {
                let mod_loader_version = config
                    .runtime
                    .mod_loader_version
                    .as_ref()
                    .ok_or(Error::InvalidInstanceConfig)?;
                let minecraft_version = &config.runtime.minecraft;
                Ok(match mod_loader_type {
                    ModLoaderType::Fabric => {
                        format!("fabric-loader-{mod_loader_version}-{minecraft_version}")
                    }
                    ModLoaderType::Quilt => {
                        format!("quilt-loader-{mod_loader_version}-{minecraft_version}",)
                    }
                    ModLoaderType::Forge => {
                        format!("{minecraft_version}-forge-{mod_loader_version}",)
                    }
                    ModLoaderType::Neoforge => {
                        format!("neoforge-{mod_loader_version}",)
                    }
                })
            })
            .unwrap_or(Ok(config.runtime.minecraft.clone()))
    }
}

pub fn calculate_playtime(instance_id: Uuid) -> Result<u64> {
    let instance_root = DATA_LOCATION.get_instance_root(&instance_id);
    let logs_root = instance_root.join("logs");
    let total_play_time: u64 = std::fs::read_dir(logs_root)?
        .filter_map(|entry| {
            let entry = match entry {
                Err(_) => return None,
                Ok(entry) => entry,
            };
            let path = entry.path();
            if path.is_file() { Some(path) } else { None }
        })
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|path| match try_read_log_dir_entry(path) {
            Err(_) => 0,
            Ok(x) => x.unwrap_or_default(),
        })
        .sum();
    Ok(total_play_time)
}

fn try_read_log_dir_entry(path: PathBuf) -> Result<Option<u64>> {
    let file = std::fs::File::open(&path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    let mut first_time: Option<u64> = None;
    let mut last_time: Option<u64> = None;

    for line in reader.lines() {
        let line = line?;

        let Some(time) = parse_log_time(&line) else {
            continue;
        };

        match first_time {
            None => {
                first_time = Some(time);
            }
            Some(first) if time > first => {
                last_time = Some(time);
            }
            _ => {}
        }
    }
    let a = first_time.zip(last_time).map(|(start, end)| end - start);
    println!("{:#?}", path);
    println!("{:#?}", a);
    Ok(a)
}

fn parse_log_time(line: &str) -> Option<u64> {
    let bytes = line.as_bytes();
    if bytes.len() < 10
        || bytes[0] != b'['
        || bytes[3] != b':'
        || bytes[6] != b':'
        || bytes[9] != b']'
    {
        return None;
    }
    let digits = [bytes[1], bytes[2], bytes[4], bytes[5], bytes[7], bytes[8]];
    if digits.iter().any(|c| !c.is_ascii_digit()) {
        return None;
    }
    let hour = (bytes[1] - b'0') as u64 * 10 + (bytes[2] - b'0') as u64;
    let minute = (bytes[4] - b'0') as u64 * 10 + (bytes[5] - b'0') as u64;
    let second = (bytes[7] - b'0') as u64 * 10 + (bytes[8] - b'0') as u64;
    if hour >= 24 || minute >= 60 || second >= 60 {
        return None;
    }
    Some(hour * 3600 + minute * 60 + second)
}
