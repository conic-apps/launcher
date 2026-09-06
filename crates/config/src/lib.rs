// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! App configuration

use account::Account;
use folder::DATA_LOCATION;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use shared::SHOULD_USE_SYSTEM_PROXY;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};

pub mod download;
pub mod error;
pub mod launch;
pub mod music;

use error::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("config")
        .invoke_handler(tauri::generate_handler![
            cmd_load_config_file,
            cmd_get_default_config,
            cmd_save_config,
            cmd_get_system_language,
            cmd_set_background_image,
            cmd_remove_background_image,
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

#[command]
fn cmd_get_system_language() -> String {
    get_system_language().to_string()
}

#[command]
async fn cmd_set_background_image(path: String) -> Result<String> {
    let dest = DATA_LOCATION.root.join("background_image");
    async_fs::copy(&path, &dest).await?;
    let filename = "background_image".to_string();
    Ok(filename)
}

#[command]
async fn cmd_remove_background_image() -> Result<()> {
    let dest = DATA_LOCATION.root.join("background_image");
    if dest.exists() {
        async_fs::remove_file(&dest).await?;
    }
    Ok(())
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
        let write_back_data = toml::to_string_pretty(&config)?;
        std::fs::write(config_file_path, write_back_data)?;
        info!("Loaded config from file");
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
///
/// The serialized values (`stable`, `beta`, `nightly`) match the update server
/// channel slugs. Old configuration files that still use `Release`, `Snapshot`
/// or `Weekly` are migrated transparently via serde aliases.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// Nightly builds, potentially unstable.
    #[serde(alias = "Weekly")]
    Nightly,
    /// Official release builds.
    #[default]
    #[serde(alias = "Release")]
    Stable,
    /// Beta builds for testing.
    #[serde(alias = "Snapshot")]
    Beta,
}

impl UpdateChannel {
    /// The channel slug used by the update server URLs.
    pub const fn as_str(&self) -> &'static str {
        match self {
            UpdateChannel::Nightly => "nightly",
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
        }
    }
}

/// Configuration options related to accessibility.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilityConfig {
    /// Whether to show reminders for new releases.
    pub release_reminder: bool,

    /// Whether to show reminders for new snapshots.
    pub snapshot_reminder: bool,

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
            release_reminder: false,
            snapshot_reminder: false,
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

    /// Whether the 3D background camera keeps moving forward.
    pub background_camera_move: bool,

    /// Whether the background follows the mouse cursor (parallax).
    pub background_parallax: bool,

    /// Custom launcher background image filename (stored in data directory).
    pub background_image: Option<String>,

    /// How much the custom background image is darkened, in percent (0-100).
    pub background_darkness: u8,
}

impl Default for AppearanceConfig {
    /// Returns the default appearance configuration.
    fn default() -> Self {
        Self {
            palette_follow_system: true,
            palette: "Mocha".to_string(),
            background_camera_move: true,
            background_parallax: true,
            background_image: None,
            background_darkness: 0,
        }
    }
}

/// The main application configuration structure.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Whether automatic updates are enabled.
    pub auto_update: bool,

    /// The account identifier and type of the currently selected account.
    /// For yggdrasil account, the identifier is account_key
    /// For Microsoft and offline account, the identifier is its profile UUID.
    pub current_account: Option<Account>,

    /// Appearance-related settings.
    pub appearance: AppearanceConfig,

    /// Accessibility-related settings.
    pub accessibility: AccessibilityConfig,

    /// The UI language code (e.g., "en_us").
    pub language: Option<String>,

    /// The selected update channel.
    pub update_channel: UpdateChannel,

    /// Java runtime executable paths the launcher must not use.
    pub disabled_java_runtime: Vec<String>,

    /// Prefer the Mojang-provided Java runtime bundled with the game.
    pub prefer_mojang_java: bool,

    /// Launch-related configuration.
    pub launch: launch::LaunchConfig,

    /// Download-related configuration.
    pub download: download::DownloadConfig,

    /// Music-related configuration.
    pub music: music::MusicConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            current_account: None,
            auto_update: true,
            language: None,
            update_channel: UpdateChannel::default(),
            disabled_java_runtime: Vec::new(),
            prefer_mojang_java: true,
            launch: launch::LaunchConfig::default(),
            download: download::DownloadConfig::default(),
            music: music::MusicConfig::default(),
        }
    }
}

pub fn get_system_language() -> &'static str {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());

    let locale = locale.replace('_', "-");
    let parts: Vec<&str> = locale.split('-').collect();

    let language = parts
        .first()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let script = parts
        .iter()
        .find(|p| p.len() == 4)
        .map(|s| s.to_ascii_lowercase());

    let region = parts
        .iter()
        .find(|p| p.len() == 2 || p.len() == 3)
        .map(|s| s.to_ascii_lowercase());

    match language.as_str() {
        "zh" => match script.as_deref() {
            Some("hant") => "zh_tw",
            Some("hans") => "zh_cn",
            _ => match region.as_deref() {
                Some("tw") | Some("hk") | Some("mo") => "zh_tw",
                _ => "zh_cn",
            },
        },
        "en" => "en_us",
        "ja" => "ja_jp",
        "ko" => "ko_kr",
        "de" => "de_de",
        "fr" => "fr_fr",
        "es" => "es_es",
        "pt" => match region.as_deref() {
            Some("br") => "pt_br",
            _ => "pt_br",
        },
        "ru" => "ru_ru",
        "tr" => "tr_tr",
        "pl" => "pl_pl",
        _ => "en_us",
    }
}
