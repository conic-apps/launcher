// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{account::get_accounts, Storage, DATA_LOCATION};

/// Module for download configuration.
pub mod download;
/// Module for game instance configuration.
pub mod instance;
/// Module for launch settings and parameters.
pub mod launch;

/// Represents the update channel selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum UpdateChannel {
    /// Weekly builds, potentially unstable.
    Weekly,
    /// Official release builds.
    Release,
    /// Snapshot builds for testing.
    Snapshot,
}

impl Default for UpdateChannel {
    /// Returns the default update channel, which is `Release`.
    fn default() -> Self {
        Self::Release
    }
}

/// Configuration options related to accessibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct AccessibilityConfig {
    /// Whether to show reminders for new releases.
    #[serde(default = "default_release_reminder")]
    pub release_reminder: bool,

    /// Whether to show reminders for new snapshots.
    #[serde(default = "default_snapshot_reminder")]
    pub snapshot_reminder: bool,

    /// Whether to hide the latest release instance.
    #[serde(default = "default_hide_latest_release")]
    pub hide_latest_release: bool,

    /// Whether to hide the latest snapshot instance.
    #[serde(default = "default_hide_latest_snapshot")]
    pub hide_latest_snapshot: bool,

    /// Whether to changing the game language to local language on first time.
    #[serde(default = "default_change_game_language")]
    pub change_game_language: bool,

    /// Whether to disable UI animations.
    #[serde(default = "default_disable_animations")]
    pub disable_animations: bool,

    /// Whether to enable high contrast mode.
    #[serde(default = "default_high_contrast_mode")]
    pub high_contrast_mode: bool,
}

impl Default for AccessibilityConfig {
    /// Returns the default values for accessibility configuration.
    fn default() -> Self {
        Self {
            release_reminder: default_release_reminder(),
            snapshot_reminder: default_snapshot_reminder(),
            hide_latest_release: default_hide_latest_release(),
            hide_latest_snapshot: default_hide_latest_snapshot(),
            change_game_language: default_change_game_language(),
            disable_animations: default_disable_animations(),
            high_contrast_mode: default_high_contrast_mode(),
        }
    }
}

/// Configuration options related to UI appearance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct AppearanceConfig {
    /// Theme name, e.g., "dark".
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for AppearanceConfig {
    /// Returns the default appearance configuration.
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

/// The main application configuration structure.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether automatic updates are enabled.
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,

    /// The UUID of the currently selected account.
    #[serde(default = "default_current_account")]
    pub current_account: String,

    /// Appearance-related settings.
    #[serde(default)]
    pub appearance: AppearanceConfig,

    /// Accessibility-related settings.
    #[serde(default)]
    pub accessibility: AccessibilityConfig,

    /// The UI language code (e.g., "en_us").
    #[serde(default = "default_language")]
    pub language: String,

    /// The selected update channel.
    #[serde(default)]
    pub update_channel: UpdateChannel,

    /// Launch-related configuration.
    #[serde(default)]
    pub launch: launch::LaunchConfig,

    /// Download-related configuration.
    #[serde(default)]
    pub download: download::DownloadConfig,
}

fn default_auto_update() -> bool {
    true
}

impl Default for Config {
    /// Returns the default configuration, using system locale and the first available account.
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

/// Returns the system locale as the default language.
fn default_language() -> String {
    sys_locale::get_locale().unwrap()
}

/// Returns the UUID of the first account, or a dummy UUID if none exists.
fn default_current_account() -> String {
    match get_accounts().unwrap().first() {
        Some(x) => x.to_owned().profile.uuid,
        None => "00000000-0000-0000-0000-000000000000".to_string(),
    }
}

/// Saves the current configuration to the configuration file.
#[tauri::command]
pub fn save_config(storage: tauri::State<'_, Storage>) {
    let data = toml::to_string_pretty(&storage.config.lock().unwrap().clone()).unwrap();
    let config_file_path = &DATA_LOCATION.config;
    std::fs::write(config_file_path, data).unwrap();
    debug!("Saved config to file");
}

/// Reads the configuration file from disk.
///
/// If the file does not exist, a default configuration is generated and saved.
///
/// # Returns
///
/// The loaded or default configuration.
#[tauri::command]
pub fn read_config_file() -> Config {
    let config_file_path = &DATA_LOCATION.config;
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

/// Updates the in-memory configuration with a new value received from the frontend.
///
/// # Arguments
///
/// * `config` - The new configuration to apply.
#[tauri::command]
pub fn update_config(storage: tauri::State<'_, Storage>, config: Config) {
    let mut storage_config = storage.config.lock().unwrap();
    *storage_config = config;
    debug!("Synchronized configuration with frontend");
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

fn default_disable_animations() -> bool {
    false
}

fn default_high_contrast_mode() -> bool {
    false
}
