// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use base64::{engine::general_purpose, Engine};
use mods::ResolvedMod;
use resourcepack::Resourcepack;
use saves::level::LevelData;
use serde::{Deserialize, Serialize};

use crate::{instance::check_instance_existance, Storage, DATA_LOCATION};

pub mod mods;
pub mod resourcepack;
pub mod saves;

#[tauri::command(async)]
pub async fn scan_mod_folder(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<Vec<ResolvedMod>, ()> {
    let instance_name = storage.current_instance.lock().unwrap().clone();

    async fn scan(
        instance_name: String,
        storage: tauri::State<'_, Storage>,
    ) -> anyhow::Result<Vec<ResolvedMod>> {
        let data_location = DATA_LOCATION.get().unwrap();
        if !check_instance_existance(instance_name.clone()).await {
            return Err(anyhow::anyhow!("Invalid instance_name"));
        }
        let mod_packs_root = data_location.get_modpacks_root(&instance_name);
        tokio::fs::create_dir_all(&mod_packs_root).await?;

        let mut folder_entries = tokio::fs::read_dir(mod_packs_root).await?;
        let mut results = Vec::new();
        while let Some(entry) = folder_entries.next_entry().await? {
            let file_type = match entry.file_type().await {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };
            let active_instance = storage.current_instance.lock().unwrap().clone();
            if active_instance != instance_name {
                return Err(anyhow::anyhow!("Current instance changed"));
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if path.metadata().is_err() {
                continue;
            }
            let parser_task =
                tokio::task::spawn_blocking(|| crate::game_data::mods::parse_mod(path));

            results.push(match parser_task.await {
                Err(_) => continue,
                Ok(result) => match result {
                    Err(_) => continue,
                    Ok(result) => result,
                },
            });
        }
        Ok(results)
    }
    match scan(instance_name, storage).await {
        Ok(results) => Ok(results
            .into_iter()
            .filter(|v| v.version.is_some())
            .collect()),
        Err(_) => Err(()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saves {
    pub icon: String,
    #[serde(rename = "levelData")]
    pub level_data: LevelData,
    #[serde(rename = "folderName")]
    pub folder_name: String,
}

#[tauri::command(async)]
pub async fn scan_saves_folder(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<Vec<Saves>, ()> {
    let instance_name = storage.current_instance.lock().unwrap().clone();

    async fn scan(
        instance_name: String,
        storage: tauri::State<'_, Storage>,
    ) -> anyhow::Result<Vec<Saves>> {
        let data_location = DATA_LOCATION.get().unwrap();
        if !check_instance_existance(instance_name.clone()).await {
            return Err(anyhow::anyhow!("Instance name is invalid"));
        }
        let saves_root = data_location.get_saves_root(&instance_name);
        tokio::fs::create_dir_all(&saves_root).await?;

        let mut folder_entries = tokio::fs::read_dir(saves_root).await?;
        let mut results = Vec::new();
        while let Some(entry) = folder_entries.next_entry().await? {
            let file_type = match entry.file_type().await {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };
            let active_instance = storage.current_instance.lock().unwrap().clone();
            if active_instance != instance_name {
                return Err(anyhow::anyhow!("Current instance changed"));
            }
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            if path.metadata().is_err() {
                continue;
            }
            let level_path = path.join("level.dat");
            let parser_task = tokio::task::spawn_blocking(|| {
                crate::game_data::saves::level::get_level_data(level_path)
            });
            let icon_path = path.join("icon.png");
            let icon = match tokio::fs::read(icon_path).await {
                Err(_) => "".to_string(),
                Ok(content) => format!(
                    "data:image/png;base64,{}",
                    general_purpose::STANDARD_NO_PAD.encode(content)
                ),
            };
            let level_data = match parser_task.await {
                Err(_) => continue,
                Ok(result) => match result {
                    Err(_) => continue,
                    Ok(result) => result,
                },
            };
            results.push(Saves {
                icon,
                level_data,
                folder_name: path.file_name().unwrap().to_string_lossy().to_string(),
            });
        }
        Ok(results)
    }
    match scan(instance_name, storage).await {
        Ok(results) => Ok(results),
        Err(_) => Err(()),
    }
}

#[tauri::command(async)]
pub async fn scan_resourcepack_folder(
    storage: tauri::State<'_, Storage>,
) -> Result<Vec<Resourcepack>, ()> {
    let instance_name = storage.current_instance.lock().unwrap().clone();
    async fn scan(
        instance_name: String,
        storage: tauri::State<'_, Storage>,
    ) -> anyhow::Result<Vec<Resourcepack>> {
        let data_location = DATA_LOCATION.get().unwrap();
        if !check_instance_existance(instance_name.clone()).await {
            return Err(anyhow::anyhow!("Instance name is invalid"));
        }
        let resourcespacks_root = data_location.get_resourcespacks_root(&instance_name);
        tokio::fs::create_dir_all(&resourcespacks_root).await?;

        let mut folder_entries = tokio::fs::read_dir(resourcespacks_root).await?;
        let mut results = Vec::new();
        while let Some(entry) = folder_entries.next_entry().await? {
            let file_type = match entry.file_type().await {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };
            let active_instance = storage.current_instance.lock().unwrap().clone();
            if active_instance != instance_name {
                return Err(anyhow::anyhow!("Current instance changed"));
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if path.metadata().is_err() {
                continue;
            }
            results.push(match resourcepack::parse_resourcepack(&path) {
                Err(_) => continue,
                Ok(result) => result,
            });
        }
        Ok(results)
    }
    match scan(instance_name, storage).await {
        Ok(x) => Ok(x),
        Err(_) => Err(()),
    }
}
