// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use crate::{account::get_accounts, Storage, DATA_LOCATION};

pub mod download;
pub mod instance;
pub mod launch;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum UpdateChannel {
    Weekly,
    Release,
    Snapshot,
}

impl Default for UpdateChannel {
    fn default() -> Self {
        Self::Release
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct AccessibilityConfig {
    #[serde(default = "default_release_reminder")]
    pub release_reminder: bool,
    #[serde(default = "default_snapshot_reminder")]
    pub snapshot_reminder: bool,
    #[serde(default = "default_hide_latest_release")]
    pub hide_latest_release: bool,
    #[serde(default = "default_hide_latest_snapshot")]
    pub hide_latest_snapshot: bool,
    #[serde(default = "default_change_game_language")]
    pub change_game_language: bool,
    #[serde(default = "default_open_log_viewer")]
    pub open_log_viewer: bool,
    #[serde(default = "default_disable_animations")]
    pub disable_animations: bool,
    #[serde(default = "default_high_contrast_mode")]
    pub high_contrast_mode: bool,
}

fn default_release_reminder() -> bool {
    true
}

fn default_snapshot_reminder() -> bool {
    true
}

fn default_hide_latest_release() -> bool {
    false
}

fn default_hide_latest_snapshot() -> bool {
    false
}

fn default_change_game_language() -> bool {
    true
}

fn default_open_log_viewer() -> bool {
    false
}

fn default_disable_animations() -> bool {
    false
}

fn default_high_contrast_mode() -> bool {
    false
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            release_reminder: default_release_reminder(),
            snapshot_reminder: default_snapshot_reminder(),
            hide_latest_release: default_hide_latest_release(),
            hide_latest_snapshot: default_hide_latest_snapshot(),
            change_game_language: default_change_game_language(),
            open_log_viewer: default_open_log_viewer(),
            disable_animations: default_disable_animations(),
            high_contrast_mode: default_high_contrast_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct AppearanceConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Config {
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
    #[serde(default = "default_current_account")]
    pub current_account: String,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub accessibility: AccessibilityConfig,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub update_channel: UpdateChannel,
    #[serde(default)]
    pub launch: launch::LaunchConfig,
    #[serde(default)]
    pub download: download::DownloadConfig,
}

fn default_auto_update() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap();
        info!("System locale is {}", locale);
        let accounts = get_accounts().unwrap();
        Self {
            appearance: AppearanceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            current_account: match accounts.first() {
                Some(x) => x.to_owned().profile.uuid,
                None => "00000000-0000-0000-0000-000000000000".to_string(),
            },
            auto_update: true,
            language: locale.replace("-", "_").to_lowercase(),
            update_channel: UpdateChannel::Release,
            launch: launch::LaunchConfig::default(),
            download: download::DownloadConfig::default(),
        }
    }
}

/// Get system locale for serde default value
fn default_language() -> String {
    sys_locale::get_locale().unwrap()
}

fn default_current_account() -> String {
    match get_accounts().unwrap().first() {
        Some(x) => x.to_owned().profile.uuid,
        None => "00000000-0000-0000-0000-000000000000".to_string(),
    }
}

#[tauri::command]
pub fn save_config(storage: tauri::State<'_, Storage>) {
    let data = toml::to_string_pretty(&storage.config.lock().unwrap().clone()).unwrap();
    let config_file_path = &DATA_LOCATION.get().unwrap().config;
    std::fs::write(config_file_path, data).unwrap();
    debug!("Saved config to file");
}

#[tauri::command]
pub fn read_config_file() -> Config {
    let config_file_path = &DATA_LOCATION.get().unwrap().config;
    error!("Loading Config");
    if !config_file_path.exists() {
        info!("No config file, using default config");
        let default_config = Config::default();
        let data = toml::to_string_pretty(&default_config).unwrap();
        std::fs::write(config_file_path, data).unwrap();
        return default_config;
    }
    let data = std::fs::read(config_file_path).expect("Could not read the config file!");
    info!("Loaded config from file");
    let result = toml::from_str::<Config>(&String::from_utf8(data).unwrap()).unwrap();
    let write_back_data = toml::to_string_pretty(&result).unwrap();
    std::fs::write(config_file_path, write_back_data).unwrap();
    result
}

#[tauri::command]
pub fn update_config(storage: tauri::State<'_, Storage>, config: Config) {
    let mut storage_config = storage.config.lock().unwrap();
    *storage_config = config;
    debug!("Configuration was synchronized with the front end");
}
