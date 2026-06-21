// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! App configuration

use account::AccountType;
use folder::DATA_LOCATION;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use shared::SHOULD_USE_SYSTEM_PROXY;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;

pub mod download;
pub mod error;
pub mod launch;

use error::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("config")
        .invoke_handler(tauri::generate_handler![
            cmd_load_config_file,
            cmd_get_default_config,
            cmd_save_config
        ])
        .build()
}

#[command]
fn cmd_load_config_file() -> Result<Config> {
    let config = load_config_file()?;
    let _ = SHOULD_USE_SYSTEM_PROXY.set(config.download.use_system_proxy);
    Ok(config)
}

#[command]
fn cmd_get_default_config() -> Config {
    Config::default()
}

#[command]
fn cmd_save_config(config: Config) -> Result<()> {
    save_config(config)
}

/// Reads the configuration file from disk.
///
/// If the file does not exist, a default configuration is generated and saved.
///
/// # Returns
///
/// The loaded or default configuration.
pub fn load_config_file() -> Result<Config> {
    let config_file_path = &DATA_LOCATION.config;
    if !config_file_path.exists() {
        info!("No config file, using default config");
        return reset_config();
    }
    let data = match std::fs::read_to_string(config_file_path) {
        Ok(x) => x,
        Err(_) => {
            error!("Could not read config file, reset it");
            return reset_config();
        }
    };
    if let Ok(config) = toml::from_str::<Config>(&data) {
        info!("Loaded config from file");
        let write_back_data = toml::to_string_pretty(&config)?;
        std::fs::write(config_file_path, write_back_data)?;
        Ok(config)
    } else {
        error!("Config file is not a toml file, reset it");
        reset_config()
    }
}

pub fn reset_config() -> Result<Config> {
    let config_file_path = &DATA_LOCATION.config;
    let default_config = Config::default();
    let data = toml::to_string_pretty(&default_config)?;
    std::fs::write(config_file_path, data)?;
    Ok(default_config)
}

/// Saves the current configuration to the configuration file.
pub fn save_config(config: Config) -> Result<()> {
    let data = toml::to_string_pretty(&config)?;
    let config_file_path = &DATA_LOCATION.config;
    std::fs::write(config_file_path, data)?;
    debug!("Saved config to file");
    Ok(())
}

/// Represents the update channel selection.
#[derive(Clone, Serialize, Deserialize, Default)]
pub enum UpdateChannel {
    /// Weekly builds, potentially unstable.
    Weekly,
    /// Official release builds.
    #[default]
    Release,
    /// Snapshot builds for testing.
    Snapshot,
}

/// Configuration options related to accessibility.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilityConfig {
    /// Whether to show reminders for new releases.
    pub release_reminder: bool,

    /// Whether to show reminders for new snapshots.
    pub snapshot_reminder: bool,

    /// Whether to hide the latest release instance.
    pub hide_latest_release: bool,

    /// Whether to hide the latest snapshot instance.
    pub hide_latest_snapshot: bool,

    /// Whether to changing the game language to local language on first time.
    pub change_game_language: bool,

    /// Whether to disable UI animations.
    pub disable_animations: bool,

    /// Whether to enable high contrast mode.
    pub high_contrast_mode: bool,
}

impl Default for AccessibilityConfig {
    /// Returns the default values for accessibility configuration.
    fn default() -> Self {
        Self {
            release_reminder: true,
            snapshot_reminder: true,
            hide_latest_release: false,
            hide_latest_snapshot: false,
            change_game_language: true,
            disable_animations: false,
            high_contrast_mode: false,
        }
    }
}

/// Configuration options related to UI appearance.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// If dark, the program will use mocha, else use latte.
    pub palette_follow_system: bool,

    /// Palette name, support mocha frappe latte macchiato.
    pub palette: String,
}

impl Default for AppearanceConfig {
    /// Returns the default appearance configuration.
    fn default() -> Self {
        Self {
            palette_follow_system: true,
            palette: "Mocha".to_string(),
        }
    }
}

/// The main application configuration structure.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Whether automatic updates are enabled.
    pub auto_update: bool,

    /// The UUID of the currently selected account.
    pub current_account_uuid: Uuid,

    /// The UUID of the currently selected account.
    pub current_account_type: AccountType,

    /// Appearance-related settings.
    pub appearance: AppearanceConfig,

    /// Accessibility-related settings.
    pub accessibility: AccessibilityConfig,

    /// The UI language code (e.g., "en_us").
    pub language: String,

    /// The selected update channel.
    pub update_channel: UpdateChannel,

    /// Launch-related configuration.
    pub launch: launch::LaunchConfig,

    /// Download-related configuration.
    pub download: download::DownloadConfig,
}

impl Default for Config {
    /// Returns the default configuration, using system locale and the first available account.
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            current_account_uuid: {
                match account::microsoft::list_accounts()
                    .unwrap_or_default()
                    .first()
                {
                    Some(x) => x.to_owned().profile.uuid,
                    None => uuid::uuid!("00000000-0000-0000-0000-000000000000"),
                }
            },
            current_account_type: AccountType::Microsoft,
            auto_update: true,
            language: get_system_language(),
            update_channel: UpdateChannel::default(),
            launch: launch::LaunchConfig::default(),
            download: download::DownloadConfig::default(),
        }
    }
}

fn get_system_language() -> String {
    let locale = sys_locale::get_locale().unwrap_or("en-US".to_string());
    if locale.contains(".") {
        locale
            .split(".")
            .collect::<Vec<_>>()
            .first()
            .expect("Could not understand system locale string")
            .to_string()
            .replace("-", "_")
            .to_lowercase()
    } else {
        locale.replace("-", "_").to_lowercase()
    }
}
