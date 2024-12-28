// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CRUD implementation for game instance

use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::instance::{InstanceConfig, ModLoaderType};
use crate::version::VersionManifest;
use crate::{Storage, DATA_LOCATION};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

static LATEST_RELEASE_INSTANCE_NAME: &str = "Latest Release";
static LATEST_SNAPSHOT_INSTANCE_NAME: &str = "Latest Snapshot";

#[derive(Deserialize, Serialize, Default)]
pub struct Instance {
    pub config: InstanceConfig,
    pub installed: bool,
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
                ModLoaderType::Neoforge => {
                    format!(
                        "neoforge-{}",
                        config.runtime.mod_loader_version.as_ref().unwrap()
                    )
                }
            },
            None => config.runtime.minecraft.to_string(),
        }
    }
}

#[tauri::command(async)]
pub async fn create_instance(config: InstanceConfig) -> Instance {
    #[allow(clippy::unwrap_used)]
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

#[derive(Deserialize)]
pub enum SortBy {
    Name,
}

#[tauri::command(async)]
pub async fn read_all_instances(sort_by: SortBy) -> Vec<Instance> {
    let instances_folder = &DATA_LOCATION.instances;
    tokio::fs::create_dir_all(instances_folder).await.unwrap();
    let mut folder_entries = tokio::fs::read_dir(instances_folder).await.unwrap();
    let mut latest_release_instance = None;
    let mut latest_snapshot_instance = None;
    let mut user_created_instance = Vec::new();
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
        debug!("Checking {}", folder_name);
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
        if instance.config.name == LATEST_RELEASE_INSTANCE_NAME {
            latest_release_instance = Some(instance);
        } else if instance.config.name == LATEST_SNAPSHOT_INSTANCE_NAME {
            latest_snapshot_instance = Some(instance);
        } else {
            user_created_instance.push(instance)
        }
    }
    let mut version_manifest = None;
    if latest_release_instance.is_none() {
        if version_manifest.is_none() {
            version_manifest = Some(VersionManifest::new().await.unwrap());
        };
        #[allow(clippy::unwrap_used)]
        let instance_config = InstanceConfig::new(
            LATEST_RELEASE_INSTANCE_NAME,
            &version_manifest.as_ref().unwrap().latest.release,
        );
        let instance = create_instance(instance_config).await;
        latest_release_instance = Some(instance);
    };
    if latest_snapshot_instance.is_none() {
        if version_manifest.is_none() {
            version_manifest = Some(VersionManifest::new().await.unwrap());
        };
        #[allow(clippy::unwrap_used)]
        let instance_config = InstanceConfig::new(
            LATEST_SNAPSHOT_INSTANCE_NAME,
            &version_manifest.unwrap().latest.snapshot,
        );
        let instance = create_instance(instance_config).await;
        latest_snapshot_instance = Some(instance);
    }
    let mut result = vec![
        latest_release_instance.unwrap(),
        latest_snapshot_instance.unwrap(),
    ];
    match sort_by {
        SortBy::Name => {
            user_created_instance.sort_by_key(|instance| instance.config.name.clone());
        }
    }
    result.extend(user_created_instance);
    result
}

#[tauri::command]
pub async fn update_instance(config: InstanceConfig, id: Uuid) {
    #[allow(clippy::unwrap_used)]
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

#[tauri::command]
pub async fn delete_instance(instance_id: Uuid) {
    tokio::fs::remove_dir_all(DATA_LOCATION.get_instance_root(&instance_id))
        .await
        .unwrap();
    info!("Deleted {}", instance_id);
}

#[tauri::command]
/// The program use a global storage to store the current instance. TODO: remove it
// TODO: remove it to support global search or commands
pub fn set_current_instance(instance: Instance, storage: tauri::State<Storage>) {
    let mut current_instance = storage.current_instance.lock().unwrap();
    debug!("Selected {}", instance.config.name);
    *current_instance = instance;
}
