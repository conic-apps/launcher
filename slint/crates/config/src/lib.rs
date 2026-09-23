// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! App configuration (Tauri-free mirror of `crates/config`).
//!
//! The Slint app shares the same `~/.conic[-debug]` data directory as the
//! Tauri app, so the same `config.toml` is read and written by both. Unknown
//! keys (e.g. `current_account`, which the Slint app does not model yet) are
//! preserved across a load/save round-trip via [`Config::extra`].
//!
//! Everything is synchronous: the Slint host loads the config before building
//! the UI and writes it back from a debounced timer.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use slint_platform::{OsFamily, PLATFORM_INFO};

pub mod download;
pub mod error;
pub mod launch;
pub mod music;

pub use error::{Error, Result};

/// Root of the launcher data directory (`~/.conic` in release, `~/.conic-debug`
/// in debug builds; `%APPDATA%` on Windows). Mirrors
/// `crates/folder::DataLocation::default`.
pub fn data_root() -> PathBuf {
    #[cfg(not(debug_assertions))]
    let default_name = "conic";
    #[cfg(debug_assertions)]
    let default_name = "conic-debug";

    let name = std::env::var("CONIC_DATA_NAME").unwrap_or_else(|_| default_name.to_string());

    match &PLATFORM_INFO.os_family {
        OsFamily::Windows => {
            PathBuf::from(std::env::var("APPDATA").expect("Could not found APP_DATA directory"))
                .join(name)
        }
        OsFamily::Linux => {
            let home = std::env::var("HOME").expect("Could not found home");
            PathBuf::from(home).join(format!(".{name}"))
        }
        OsFamily::Macos => {
            let home = std::env::var("HOME").expect("Could not found home");
            PathBuf::from(home).join(name)
        }
    }
}

/// Path of `config.toml` inside [`data_root`].
pub fn config_path() -> PathBuf {
    data_root().join("config.toml")
}

/// The launcher's music folder.
pub fn music_dir() -> PathBuf {
    data_root().join("music")
}

/// The launcher's log folder.
pub fn logs_dir() -> PathBuf {
    data_root().join("logs")
}

/// Path of the copied custom background image.
pub fn background_image_path() -> PathBuf {
    data_root().join("background_image")
}

/// Reads the configuration file from disk.
///
/// If the file does not exist, a default configuration is generated and saved.
pub fn load_config_file() -> Result<Config> {
    let config_file_path = config_path();
    if !config_file_path.exists() {
        info!("No config file, using default config");
        return reset_config();
    }
    let data = match std::fs::read_to_string(&config_file_path) {
        Ok(x) => x,
        Err(_) => {
            error!("Could not read config file, reset it");
            return reset_config();
        }
    };
    if let Ok(config) = toml::from_str::<Config>(&data) {
        let write_back_data = toml::to_string_pretty(&config)?;
        std::fs::write(&config_file_path, write_back_data)?;
        info!("Loaded config from file");
        Ok(config)
    } else {
        error!("Config file is not a toml file, reset it");
        reset_config()
    }
}

/// Writes a default configuration to disk and returns it.
pub fn reset_config() -> Result<Config> {
    let default_config = Config::default();
    save_config(&default_config)?;
    Ok(default_config)
}

/// Saves the configuration to the configuration file.
pub fn save_config(config: &Config) -> Result<()> {
    let config_file_path = config_path();
    if let Some(parent) = config_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = toml::to_string_pretty(config)?;
    std::fs::write(&config_file_path, data)?;
    debug!("Saved config to file");
    Ok(())
}

/// Copies `path` to the data directory as the custom background image,
/// returning the stored file name.
pub fn set_background_image(path: &Path) -> Result<String> {
    let dest = background_image_path();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(path, &dest)?;
    Ok("background_image".to_string())
}

/// Removes the stored custom background image, if any.
pub fn remove_background_image() -> Result<()> {
    let dest = background_image_path();
    if dest.exists() {
        std::fs::remove_file(&dest)?;
    }
    Ok(())
}

/// Update channel selection. Serialized values match the update server slugs.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[serde(alias = "Weekly")]
    Nightly,
    #[default]
    #[serde(alias = "Release")]
    Stable,
    #[serde(alias = "Snapshot")]
    Beta,
}

impl UpdateChannel {
    pub const fn as_str(&self) -> &'static str {
        match self {
            UpdateChannel::Nightly => "nightly",
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
        }
    }

    pub fn from_slug(value: &str) -> Self {
        match value {
            "nightly" => UpdateChannel::Nightly,
            "beta" => UpdateChannel::Beta,
            _ => UpdateChannel::Stable,
        }
    }
}

/// Configuration options related to accessibility.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilityConfig {
    pub release_reminder: bool,
    pub snapshot_reminder: bool,
    pub change_game_language: bool,
    pub disable_animations: bool,
    pub high_contrast_mode: bool,
}

impl Default for AccessibilityConfig {
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
    pub palette_follow_system: bool,
    pub palette: String,
    pub background_camera_move: bool,
    pub background_parallax: bool,
    pub background_image: Option<String>,
    pub background_darkness: u8,
}

impl Default for AppearanceConfig {
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

/// The main application configuration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auto_update: bool,
    pub appearance: AppearanceConfig,
    pub accessibility: AccessibilityConfig,
    pub language: Option<String>,
    pub update_channel: UpdateChannel,
    pub disabled_java_runtime: Vec<String>,
    pub prefer_mojang_java: bool,
    pub launch: launch::LaunchConfig,
    pub download: download::DownloadConfig,
    pub music: music::MusicConfig,

    /// Unknown keys (e.g. `current_account`) kept so a round-trip does not drop
    /// data written by the Tauri app.
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_update: true,
            appearance: AppearanceConfig::default(),
            accessibility: AccessibilityConfig::default(),
            language: None,
            update_channel: UpdateChannel::default(),
            disabled_java_runtime: Vec::new(),
            prefer_mojang_java: true,
            launch: launch::LaunchConfig::default(),
            download: download::DownloadConfig::default(),
            music: music::MusicConfig::default(),
            extra: BTreeMap::new(),
        }
    }
}

/// Best-effort mapping from the system locale to a bundled launcher language.
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
        "pt" => "pt_br",
        "ru" => "ru_ru",
        "tr" => "tr_tr",
        "pl" => "pl_pl",
        _ => "en_us",
    }
}
