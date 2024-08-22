// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

use crate::{Storage, DATA_LOCATION};

pub mod instance;
pub mod launch;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Config {
    pub launch: launch::LaunchConfig,
    pub max_connection: usize,
    pub max_download_speed: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            launch: launch::LaunchConfig::default(),
            max_connection: 100,
            max_download_speed: 0,
        }
    }
}

#[tauri::command]
pub fn save_config(storage: tauri::State<'_, Storage>) {
    let data = toml::to_string_pretty(&storage.config.lock().unwrap().clone()).unwrap();
    let path = DATA_LOCATION.get().unwrap().root.join(".aml.toml");
    std::fs::write(path, data).unwrap();
}

#[tauri::command]
pub fn read_config_file() -> Config {
    let path = DATA_LOCATION.get().unwrap().root.join(".aml.toml");
    if !path.exists() {
        let default_config = Config::default();
        let data = toml::to_string_pretty(&default_config).unwrap();
        std::fs::write(&path, data).unwrap();
        return default_config;
    }
    let data = std::fs::read(path).unwrap();
    toml::from_str::<Config>(String::from_utf8(data).unwrap().as_ref()).unwrap()
}

#[tauri::command]
pub fn update_config(storage: tauri::State<'_, Storage>, config: Config) {
    let mut storage_config = storage.config.lock().unwrap();
    *storage_config = config
}
