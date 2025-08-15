// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use account::AccountType;
use folder::DATA_LOCATION;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;

pub mod download;
pub mod error;
pub mod instance;
pub mod launch;

use error::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("config")
        .js_init_script(get_init_config_script())
        .invoke_handler(tauri::generate_handler![
            cmd_load_config_file,
            cmd_save_config
        ])
        .build()
}

fn get_init_config_script() -> String {
    let config = load_config_file().unwrap_or_else(|e| {
        log::error!("FATAL: Unable to load or reset config file!");
        panic!("{e}")
    });
    "
        Object.defineProperty(window, '__CONIC_CONFIG__', {
            value: JSON.parse(`"
        .to_string()
        + serde_json::to_string_pretty(&config)
            .expect("The program is broken")
            .as_ref()
        + "`)
        })
    "
}

#[command]
fn cmd_load_config_file() -> Result<Config> {
    load_config_file()
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
    pub current_account_uuid: Uuid,

    /// The UUID of the currently selected account.
    #[serde(default = "default_current_account_type")]
    pub current_account_type: AccountType,

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
        let accounts = account::microsoft::list_accounts().unwrap_or_default();
        Self {
            appearance: AppearanceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            current_account_uuid: match accounts.first() {
                Some(x) => x.to_owned().profile.uuid,
                None => uuid::uuid!("00000000-0000-0000-0000-000000000000"),
            },
            current_account_type: AccountType::Microsoft,
            auto_update: true,
            language: default_language(),
            update_channel: UpdateChannel::default(),
            launch: launch::LaunchConfig::default(),
            download: download::DownloadConfig::default(),
        }
    }
}

fn default_language() -> String {
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

fn default_current_account() -> Uuid {
    match account::microsoft::list_accounts()
        .unwrap_or_default()
        .first()
    {
        Some(x) => x.to_owned().profile.uuid,
        None => uuid::uuid!("00000000-0000-0000-0000-000000000000"),
    }
}

fn default_current_account_type() -> AccountType {
    AccountType::Microsoft
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
