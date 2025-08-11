// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CRUD implementation for game instance

use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use config::instance::{InstanceConfig, ModLoaderType};
use folder::DATA_LOCATION;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Runtime, command};
use uuid::Uuid;

static LATEST_RELEASE_INSTANCE_NAME: &str = "Latest Release";
static LATEST_SNAPSHOT_INSTANCE_NAME: &str = "Latest Snapshot";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("instance")
        .invoke_handler(tauri::generate_handler![
            cmd_create_instance,
            cmd_list_instances,
            cmd_update_instance,
            cmd_delete_instance
        ])
        .build()
}

#[command]
async fn cmd_create_instance(config: InstanceConfig) -> Instance {
    create_instance(config).await
}

#[command]
async fn cmd_list_instances(sort_by: SortBy) -> Vec<Instance> {
    list_instances(sort_by).await
}

#[command]
async fn cmd_update_instance(config: InstanceConfig, id: Uuid) {
    update_instance(config, id).await
}

#[command]
async fn cmd_delete_instance(id: Uuid) {
    delete_instance(id).await
}

/// Creates a new game instance using the provided configuration.
pub async fn create_instance(config: InstanceConfig) -> Instance {
    let id = if config.name == LATEST_RELEASE_INSTANCE_NAME {
        uuid::Uuid::from_u128(114514)
    } else if config.name == LATEST_SNAPSHOT_INSTANCE_NAME {
        uuid::Uuid::from_u128(1919810)
    } else {
        uuid::Uuid::from_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
    };
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    let config_file_path = instance_root.join("instance.toml");
    tokio::fs::create_dir_all(
        config_file_path
            .parent()
            .ok_or(anyhow::anyhow!("Path Error"))
            .unwrap(),
    )
    .await
    .unwrap();
    tokio::fs::write(config_file_path, toml::to_string_pretty(&config).unwrap())
        .await
        .unwrap();
    info!("Created instance: {}", config.name);
    Instance {
        config,
        installed: false,
        id,
    }
}

/// Enum representing different sorting strategies for listing instances.
#[derive(Deserialize)]
pub enum SortBy {
    /// Sort by instance name.
    Name,
    // TODO: Other sort strategies, such as createdon, last played at, play frequency...
}

/// Reads all instances stored in the data directory,
/// creates default instances for latest release and snapshot if missing,
/// and returns a sorted list.
///
/// Default instances are created if not found.
pub async fn list_instances(sort_by: SortBy) -> Vec<Instance> {
    let instances_folder = &DATA_LOCATION.instances;
    tokio::fs::create_dir_all(instances_folder).await.unwrap();
    let mut folder_entries = tokio::fs::read_dir(instances_folder).await.unwrap();
    let mut instances = Vec::new();

    while let Some(entry) = folder_entries.next_entry().await.unwrap() {
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
        let config_content = match tokio::fs::read_to_string(instance_config).await {
            Err(_) => continue,
            Ok(content) => content,
        };
        let instance = Instance {
            config: match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => config,
                Err(_) => continue,
            },
            installed: matches!(
                tokio::fs::try_exists(path.join(".install.lock")).await,
                Ok(true)
            ),
            id: match uuid::Uuid::from_str(&folder_name) {
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
    instances
}

/// Updates the configuration file of an existing instance
/// specified by the given UUID.
pub async fn update_instance(config: InstanceConfig, id: Uuid) {
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    let config_file = instance_root.join("instance.toml");
    println!(
        "{:#?}",
        config.launch_config.enable_instance_specific_settings
    );
    tokio::fs::write(config_file, toml::to_string_pretty(&config).unwrap())
        .await
        .unwrap();
    info!("Updated instance: {}", config.name);
}

/// Deletes the instance directory corresponding to the given UUID.
pub async fn delete_instance(id: Uuid) {
    tokio::fs::remove_dir_all(DATA_LOCATION.get_instance_root(&id))
        .await
        .unwrap();
    info!("Deleted {id}");
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
    pub id: uuid::Uuid,
}

impl Instance {
    pub fn get_version_id(&self) -> String {
        let config = &self.config;
        match config.runtime.mod_loader_type.as_ref() {
            Some(mod_loader_type) => match mod_loader_type {
                ModLoaderType::Fabric => {
                    format!(
                        "fabric-loader-{}-{}",
                        config.runtime.mod_loader_version.as_ref().unwrap(),
                        config.runtime.minecraft
                    )
                }
                ModLoaderType::Quilt => {
                    format!(
                        "quilt-loader-{}-{}",
                        config.runtime.mod_loader_version.as_ref().unwrap(),
                        config.runtime.minecraft
                    )
                }
                ModLoaderType::Forge => {
                    format!(
                        "{}-forge-{}",
                        config.runtime.minecraft,
                        config.runtime.mod_loader_version.as_ref().unwrap()
                    )
                }
                ModLoaderType::Neoforged => {
                    format!(
                        "neoforged-{}",
                        config.runtime.mod_loader_version.as_ref().unwrap()
                    )
                }
            },
            None => config.runtime.minecraft.to_string(),
        }
    }
}
