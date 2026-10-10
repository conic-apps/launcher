// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! App configuration.
//!
//! `config.toml` lives in the launcher's data directory (`conic`, or
//! `conic-debug` for a debug build; the exact path is platform-dependent), whose
//! layout comes from [`storage::LOCATIONS`]. Every key the launcher models is
//! named here, and [`Config::extra`] is the catch-all for anything it does not,
//! kept so a load/save round-trip cannot drop a key a newer build wrote.
//!
//! Everything is synchronous: the app loads the config before building the UI
//! and writes it back from a debounced timer.

use std::{collections::BTreeMap, fs::File, path::Path, time::SystemTime};

use account::Account;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use storage::LOCATIONS;

pub mod download;
pub mod error;
pub mod launch;
pub mod music;

pub use error::{Error, Result};

/// Reads the configuration file from disk.
///
/// If the file does not exist, a default configuration is generated and saved.
pub fn load_config_file() -> Result<Config> {
    let config_file_path = &LOCATIONS.launcher.config;
    if !config_file_path.exists() {
        info!(
            "No config file at {}, using default config",
            config_file_path.display()
        );
        return reset_config();
    }
    let data = match std::fs::read_to_string(config_file_path) {
        Ok(x) => x,
        Err(error) => {
            error!(
                "Could not read {}: {error}; resetting the config",
                config_file_path.display()
            );
            return reset_config();
        }
    };
    match toml::from_str::<Config>(&data) {
        Ok(config) => {
            // The write-back canonicalises the file. A failure here leaves the
            // config in memory only, and a crash loses every setting made since.
            let write_back_data = toml::to_string_pretty(&config)?;
            if let Err(error) = std::fs::write(config_file_path, write_back_data) {
                log::warn!(
                    "{} could not be rewritten after loading: {error}; settings are held in \
                     memory only until they are saved",
                    config_file_path.display()
                );
            }
            info!("Loaded config from {}", config_file_path.display());
            Ok(config)
        }
        Err(error) => {
            error!(
                "{} is not a valid config ({error}); resetting it, which discards every \
                 setting in it",
                config_file_path.display()
            );
            reset_config()
        }
    }
}

/// Writes a default configuration to disk and returns it.
pub fn reset_config() -> Result<Config> {
    let default_config = Config::default();
    save_config(&default_config)?;
    Ok(default_config)
}

pub fn save_config(config: &Config) -> Result<()> {
    save_config_to(&LOCATIONS.launcher.config, config)
}

/// Saves `config` to an arbitrary path.
///
/// The setup wizard uses this when the user picks a launcher data directory:
/// the config has to be written to the new directory before the process
/// restarts into it, so the choice made in the wizard survives the move.
pub fn save_config_to(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = toml::to_string_pretty(config)?;
    std::fs::write(path, data)?;
    info!("Saved config to {}", path.display());
    Ok(())
}

/// Copies `path` to the data directory as the custom background image,
/// returning the stored file name.
pub fn set_background_image(path: &Path) -> Result<String> {
    let dest = LOCATIONS.launcher.root.join("background_image");
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(path, &dest)?;
    stamp_now(&dest);
    Ok("background_image".to_string())
}

/// Stamps `path` as modified now, so a replacement is a distinct revision.
///
/// `fs::copy` carries the source's timestamp over on macOS: replacing the stored
/// background with an image sharing its mtime would look unchanged to the loader
/// and the new wallpaper would not appear until a restart.
fn stamp_now(path: &Path) {
    if let Err(error) = File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(SystemTime::now()))
    {
        // Not fatal: the picture is in place, only its freshness cannot be told.
        debug!("could not stamp '{}': {error}", path.display());
    }
}

pub fn remove_background_image() -> Result<()> {
    let dest = LOCATIONS.launcher.root.join("background_image");
    if dest.exists() {
        std::fs::remove_file(&dest)?;
    }
    Ok(())
}

/// Update channel selection. Serialized as the lowercase values `nightly`,
/// `stable` and `beta`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[serde(alias = "Weekly")]
    Nightly,
    #[serde(alias = "Release")]
    Stable,
    #[serde(alias = "Snapshot")]
    Beta,
}

impl Default for UpdateChannel {
    /// The default channel follows the build: a pre-release carries its
    /// prerelease assets on the `beta` channel, so defaulting it to `stable`
    /// would strand the install on a 204 every time (see
    /// `crates/update/src/manifest.rs`). Release builds track `stable`.
    fn default() -> Self {
        let version = shared::app_version().to_ascii_lowercase();
        if ["alpha", "beta", "rc"]
            .iter()
            .any(|tag| version.contains(tag))
        {
            UpdateChannel::Beta
        } else {
            UpdateChannel::Stable
        }
    }
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auto_update: bool,
    /// Whether the first-run setup wizard has been finished or skipped.
    ///
    /// An older `config.toml` that predates this field is missing the key, so it
    /// reads as `false` and the wizard is shown once; the wizard writes it back
    /// as `true` when it finishes or is dismissed.
    pub setup_completed: bool,
    pub current_account: Option<Account>,
    pub appearance: AppearanceConfig,
    pub accessibility: AccessibilityConfig,
    pub language: Option<String>,
    pub update_channel: UpdateChannel,
    pub disabled_java_runtime: Vec<String>,
    pub prefer_mojang_java: bool,
    pub launch: launch::LaunchConfig,
    pub download: download::DownloadConfig,
    pub music: music::MusicConfig,

    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_update: true,
            setup_completed: false,
            current_account: None,
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
    let locale = sys_locale::get_locale().unwrap_or_else(|| {
        // English is the right fallback, but a locale that cannot be read is a
        // first-launch-only condition worth recording: the app then opens in
        // English on a machine that is configured for something else.
        log::debug!("the system locale could not be read; falling back to en-US");
        "en-US".to_string()
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_is_the_documented_one() {
        let config = Config::default();
        assert!(config.auto_update);
        assert!(config.prefer_mojang_java);
        assert_eq!(config.update_channel, UpdateChannel::Stable);
        assert_eq!(config.appearance.palette, "Mocha");
        assert!(config.accessibility.change_game_language);
        assert_eq!(config.launch.launcher_name, "Conic_Launcher");
        assert_eq!(config.launch.width, 854);
        assert_eq!(config.download.max_connections, 100);
        assert_eq!(config.music.main_volumn, 100);
        assert_eq!(config.music.main_volumn_background, 25);
        assert!(!config.setup_completed);
    }

    #[test]
    fn a_config_without_the_setup_flag_is_not_set_up() {
        let config: Config = toml::from_str("auto_update = false\n").expect("valid toml");
        assert!(!config.setup_completed);

        let done: Config = toml::from_str("setup_completed = true\n").expect("valid toml");
        assert!(done.setup_completed);
    }

    #[test]
    fn update_channel_maps_its_slug_and_back() {
        assert_eq!(UpdateChannel::from_slug("nightly"), UpdateChannel::Nightly);
        assert_eq!(UpdateChannel::from_slug("beta"), UpdateChannel::Beta);
        assert_eq!(UpdateChannel::from_slug("stable"), UpdateChannel::Stable);
        assert_eq!(UpdateChannel::from_slug("nonsense"), UpdateChannel::Stable);
        assert_eq!(UpdateChannel::Nightly.as_str(), "nightly");
        assert_eq!(UpdateChannel::Stable.as_str(), "stable");
        assert_eq!(UpdateChannel::Beta.as_str(), "beta");
    }

    #[test]
    fn update_channel_reads_its_legacy_aliases() {
        #[derive(Deserialize)]
        struct Wrapper {
            update_channel: UpdateChannel,
        }

        let parse = |value: &str| -> UpdateChannel {
            let wrapper: Wrapper =
                toml::from_str(&format!("update_channel = \"{value}\"")).expect("valid channel");
            wrapper.update_channel
        };
        assert_eq!(parse("Weekly"), UpdateChannel::Nightly);
        assert_eq!(parse("Release"), UpdateChannel::Stable);
        assert_eq!(parse("Snapshot"), UpdateChannel::Beta);
    }

    #[test]
    fn stamp_now_makes_a_replaced_file_look_new() {
        let dir = std::env::temp_dir().join(format!("conic-stamp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let file = dir.join("background_image");
        std::fs::write(&file, b"one").expect("a file");
        File::options()
            .write(true)
            .open(&file)
            .expect("open")
            .set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000))
            .expect("an old mtime");
        let before = std::fs::metadata(&file)
            .expect("metadata")
            .modified()
            .expect("mtime");
        stamp_now(&file);
        let after = std::fs::metadata(&file)
            .expect("metadata")
            .modified()
            .expect("mtime");
        assert!(
            after > before,
            "the replacement was not given a new revision"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_partial_config_is_filled_in_with_defaults() {
        let config: Config = toml::from_str("auto_update = false\n").expect("valid toml");
        assert!(!config.auto_update);
        // Every unmentioned section keeps its default.
        assert_eq!(config.launch.width, 854);
        assert_eq!(config.appearance.palette, "Mocha");
    }

    #[test]
    fn unknown_keys_survive_a_load_save_round_trip() {
        let config: Config = toml::from_str("a_key_from_a_newer_build = 42\n").expect("valid toml");
        assert_eq!(
            config.extra.get("a_key_from_a_newer_build"),
            Some(&toml::Value::Integer(42))
        );

        let written = toml::to_string_pretty(&config).expect("serialisable");
        assert!(written.contains("a_key_from_a_newer_build = 42"));
    }
}
