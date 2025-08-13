// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CRUD implementation for game instance

use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use config::instance::{InstanceConfig, ModLoaderType};
use folder::DATA_LOCATION;
use futures::TryStreamExt;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Runtime, command};
use uuid::Uuid;

pub mod error;

use error::*;

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
async fn cmd_create_instance(config: InstanceConfig) -> Result<()> {
    create_instance(config).await
}

#[command]
async fn cmd_list_instances(sort_by: SortBy) -> Result<Vec<Instance>> {
    list_instances(sort_by).await
}

#[command]
async fn cmd_update_instance(config: InstanceConfig, id: Uuid) -> Result<()> {
    update_instance(config, id).await
}

#[command]
async fn cmd_delete_instance(id: Uuid) -> Result<()> {
    delete_instance(id).await
}

/// Creates a new game instance using the provided configuration.
pub async fn create_instance(config: InstanceConfig) -> Result<()> {
    let id = if config.name == LATEST_RELEASE_INSTANCE_NAME {
        uuid::Uuid::from_u128(114514)
    } else if config.name == LATEST_SNAPSHOT_INSTANCE_NAME {
        uuid::Uuid::from_u128(1919810)
    } else {
        uuid::Uuid::from_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Incorrect System Time")
                .as_nanos(),
        )
    };
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    let config_file_path = instance_root.join("instance.toml");
    if let Some(parent) = config_file_path.parent() {
        async_fs::create_dir_all(parent).await?
    }
    async_fs::write(config_file_path, toml::to_string_pretty(&config)?).await?;
    info!("Created instance: {}", config.name);
    Ok(())
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
    Ok(instances)
}

/// Updates the configuration file of an existing instance
/// specified by the given UUID.
pub async fn update_instance(config: InstanceConfig, id: Uuid) -> Result<()> {
    let instance_root = DATA_LOCATION.get_instance_root(&id);
    let config_file = instance_root.join("instance.toml");
    println!(
        "{:#?}",
        config.launch_config.enable_instance_specific_settings
    );
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
    pub id: uuid::Uuid,
}

impl Instance {
    pub fn get_version_id(&self) -> Result<String> {
        let config = &self.config;
        let version_id = match config.runtime.mod_loader_type.as_ref() {
            Some(mod_loader_type) => match mod_loader_type {
                ModLoaderType::Fabric => {
                    format!(
                        "fabric-loader-{}-{}",
                        config
                            .runtime
                            .mod_loader_version
                            .as_ref()
                            .ok_or(Error::InvalidInstanceConfig)?,
                        config.runtime.minecraft
                    )
                }
                ModLoaderType::Quilt => {
                    format!(
                        "quilt-loader-{}-{}",
                        config
                            .runtime
                            .mod_loader_version
                            .as_ref()
                            .ok_or(Error::InvalidInstanceConfig)?,
                        config.runtime.minecraft
                    )
                }
                ModLoaderType::Forge => {
                    format!(
                        "{}-forge-{}",
                        config.runtime.minecraft,
                        config
                            .runtime
                            .mod_loader_version
                            .as_ref()
                            .ok_or(Error::InvalidInstanceConfig)?
                    )
                }
                ModLoaderType::Neoforged => {
                    format!(
                        "neoforged-{}",
                        config
                            .runtime
                            .mod_loader_version
                            .as_ref()
                            .ok_or(Error::InvalidInstanceConfig)?
                    )
                }
            },
            None => config.runtime.minecraft.to_string(),
        };
        Ok(version_id)
    }
}
