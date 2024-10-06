// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{
    account::{get_accounts, Account},
    Storage, DATA_LOCATION,
};

pub mod download;
pub mod instance;
pub mod launch;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum UpdateChannel {
    Weekly,
    Release,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct AccessibilityConfig {
    pub release_reminder: bool,
    pub snapshot_reminder: bool,
    pub hide_latest_release: bool,
    pub hide_latest_snapshot: bool,
    pub change_game_language: bool,
    pub open_log_viewer: bool,
    pub disable_animations: bool,
    pub high_contrast_mode: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            release_reminder: true,
            snapshot_reminder: true,
            hide_latest_release: false,
            hide_latest_snapshot: false,
            change_game_language: true,
            open_log_viewer: false,
            disable_animations: false,
            high_contrast_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct AppearanceConfig {
    pub theme: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Config {
    pub auto_update: bool,
    pub current_account: String,
    pub appearance: AppearanceConfig,
    pub accessibility: AccessibilityConfig,
    pub language: String,
    pub update_channel: UpdateChannel,
    pub launch: launch::LaunchConfig,
    pub download: download::DownloadConfig,
}

impl Default for Config {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap();
        info!("System locale is {}", locale);
        let accounts = get_accounts().unwrap();
        let default_account = match accounts.first() {
            Some(x) => x.to_owned(),
            None => {
                let default_account = Account::default();
                let path = DATA_LOCATION.get().unwrap().root.join("accounts.json");
                let contents =
                    serde_json::to_string_pretty(&vec![default_account.clone()]).unwrap();
                std::fs::write(path, contents).unwrap();
                default_account
            }
        };
        Self {
            appearance: AppearanceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            current_account: default_account.profile.uuid,
            auto_update: true,
            language: locale.replace("-", "_").to_lowercase(),
            update_channel: UpdateChannel::Release,
            launch: launch::LaunchConfig::default(),
            download: download::DownloadConfig::default(),
        }
    }
}

#[tauri::command]
pub fn save_config(storage: tauri::State<'_, Storage>) {
    let data = toml::to_string_pretty(&storage.config.lock().unwrap().clone()).unwrap();
    let path = DATA_LOCATION.get().unwrap().root.join(".aml.toml");
    std::fs::write(path, data).unwrap();
    debug!("Saved config to file");
}

#[tauri::command]
pub fn read_config_file() -> Config {
    let path = DATA_LOCATION.get().unwrap().root.join(".aml.toml");
    if !path.exists() {
        info!("No config file, using default config");
        let default_config = Config::default();
        let data = toml::to_string_pretty(&default_config).unwrap();
        std::fs::write(&path, data).unwrap();
        return default_config;
    }
    let data = std::fs::read(path).unwrap();
    debug!("Loaded config from file");
    toml::from_str::<Config>(String::from_utf8(data).unwrap().as_ref()).unwrap()
}

#[tauri::command]
pub fn update_config(storage: tauri::State<'_, Storage>, config: Config) {
    let mut storage_config = storage.config.lock().unwrap();
    *storage_config = config;
    debug!("Configuration was synchronized with the front end");
}
