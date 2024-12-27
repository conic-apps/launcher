// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CRUD implementation for game instance

use crate::config::instance::InstanceConfig;
use crate::version::VersionManifest;
use crate::{Storage, DATA_LOCATION};
use log::{debug, info};
use serde::{Deserialize, Serialize};

static LATEST_RELEASE_INSTANCE_NAME: &str = "Latest Release";
static LATEST_SNAPSHOT_INSTANCE_NAME: &str = "Latest Snapshot";

#[derive(Deserialize, Serialize, Default)]
pub struct Instance {
    pub config: InstanceConfig,
    pub installed: bool,
}

#[tauri::command(async)]
pub async fn create_instance(config: InstanceConfig) -> InstanceConfig {
    #[allow(clippy::unwrap_used)]
    let data_location = DATA_LOCATION.get().unwrap();
    let instance_root = data_location.get_instance_root(&config.name);
    let config_file = instance_root.join("instance.toml");
    tokio::fs::create_dir_all(
        config_file
            .parent()
            .ok_or(anyhow::anyhow!("Path Error"))
            .unwrap(),
    )
    .await
    .unwrap();
    tokio::fs::write(config_file, toml::to_string_pretty(&config).unwrap())
        .await
        .unwrap();
    info!("Created instance: {}", config.name);
    config
}

#[derive(Deserialize)]
pub enum SortBy {
    Name,
}

#[tauri::command(async)]
pub async fn read_all_instances(sort_by: SortBy) -> Vec<Instance> {
    let data_location = DATA_LOCATION.get().unwrap();
    let instances_folder = &data_location.instances;
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
        if metadata.len() > 2000000 || !instance_config.is_file() {
            continue;
        }
        let config_content = match tokio::fs::read_to_string(instance_config).await {
            Err(_) => continue,
            Ok(content) => content,
        };
        let config = match toml::from_str::<InstanceConfig>(&config_content) {
            Ok(config) => config,
            Err(_) => continue,
        };
        if folder_name != config.name {
            continue;
        }
        let installed = matches!(
            tokio::fs::try_exists(path.join(".install.lock")).await,
            Ok(true)
        );
        if config.name == LATEST_RELEASE_INSTANCE_NAME {
            latest_release_instance = Some(Instance { config, installed });
        } else if config.name == LATEST_SNAPSHOT_INSTANCE_NAME {
            latest_snapshot_instance = Some(Instance { config, installed });
        } else {
            user_created_instance.push(Instance { config, installed })
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
        let instance_config = create_instance(instance_config).await;
        latest_release_instance = Some(Instance {
            config: instance_config,
            installed: false,
        });
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
        let instance_config = create_instance(instance_config).await;
        latest_snapshot_instance = Some(Instance {
            config: instance_config,
            installed: false,
        });
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
pub async fn update_instance(config: InstanceConfig) {
    #[allow(clippy::unwrap_used)]
    let data_location = DATA_LOCATION.get().unwrap();
    let instance_root = data_location.get_instance_root(&config.name);
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
pub async fn delete_instance(instance_name: String) {
    info!("Deleting {}", instance_name);
    let data_location = DATA_LOCATION.get().unwrap();
    tokio::fs::remove_dir_all(data_location.get_instance_root(&instance_name))
        .await
        .unwrap();
    info!("Deleted {}", instance_name);
}

#[tauri::command]
/// The program use a global storage to store the current instance. TODO: remove it
// TODO: remove it to support global search or commands
pub fn set_current_instance(instance: Instance, storage: tauri::State<Storage>) {
    let mut current_instance = storage.current_instance.lock().unwrap();
    debug!("Selected {}", instance.config.name);
    *current_instance = instance;
}
