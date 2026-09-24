// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/instance`: CRUD for game instances.
//!
//! The original crate exposes the same operations through Tauri commands; the
//! Slint app calls these functions directly (synchronously) and owns the UI
//! state itself. Only the data model and filesystem logic live here.

use std::{
    cmp::Ordering,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use flate2::read::GzDecoder;
use log::info;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use slint_folder::DATA_LOCATION;
use uuid::Uuid;

mod config;
mod error;

pub use config::*;
pub use error::*;

/// Creates a new game instance using the provided configuration.
pub fn create_instance(config: InstanceConfig, id: Option<&str>) -> Result<String> {
    let random_uuid = Uuid::new_v4().to_string();
    let id = id.unwrap_or(&random_uuid);
    let instance_root = DATA_LOCATION.get_instance_root(id);
    let config_file_path = instance_root.join("instance.toml");
    if let Some(parent) = config_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(config_file_path, toml::to_string_pretty(&config)?)?;
    info!("Created instance: {}", config.name);
    Ok(id.to_string())
}

/// Enum representing different sorting strategies for listing instances.
#[derive(Clone, Copy, Deserialize)]
pub enum SortBy {
    /// Sort by instance name (ascending).
    Name,
    /// Sort by Minecraft version (newest first).
    Version,
    /// Sort by total play time (most played first).
    Playtime,
    /// Sort by last played date (most recent first).
    LastPlayed,
}

/// Reads all instances stored in the data directory.
pub fn list_instances(sort_by: SortBy) -> Result<Vec<Instance>> {
    let instances_folder = &DATA_LOCATION.instances;
    std::fs::create_dir_all(instances_folder)?;
    let mut instances = Vec::new();

    for entry in std::fs::read_dir(instances_folder)? {
        let entry = match entry {
            Err(_) => continue,
            Ok(entry) => entry,
        };
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let folder_name = match path.file_name() {
            None => continue,
            Some(x) => x,
        }
        .to_string_lossy()
        .to_string();
        let instance_config = path.join("instance.toml");
        let metadata = match instance_config.metadata() {
            Err(_) => continue,
            Ok(result) => result,
        };
        if metadata.len() > 2_000_000 || !instance_config.is_file() {
            continue;
        }
        let config_content = match std::fs::read_to_string(instance_config) {
            Err(_) => continue,
            Ok(content) => content,
        };
        let instance_id = folder_name;
        let instance = Instance {
            config: match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => config,
                Err(_) => continue,
            },
            installed: path.join(".install.lock").is_file(),
            last_played: get_launch_script_timestamp(&instance_id),
            id: instance_id,
            has_background: path.join("background").is_file(),
        };
        instances.push(instance);
    }
    match sort_by {
        SortBy::Name => {
            instances.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        }
        SortBy::Version => {
            instances.sort_by(|a, b| {
                compare_minecraft_versions(&b.config.runtime.minecraft, &a.config.runtime.minecraft)
            });
        }
        SortBy::Playtime => {
            let mut playtime_instances = instances
                .into_iter()
                .map(|instance| {
                    (
                        calculate_playtime(&instance.id).unwrap_or_default(),
                        instance,
                    )
                })
                .collect::<Vec<_>>();
            playtime_instances.sort_by_key(|(playtime, _)| std::cmp::Reverse(*playtime));
            instances = playtime_instances
                .into_iter()
                .map(|(_, instance)| instance)
                .collect();
        }
        SortBy::LastPlayed => {
            instances.sort_by(|a, b| {
                b.last_played
                    .unwrap_or_default()
                    .cmp(&a.last_played.unwrap_or_default())
            });
        }
    }
    info!("Loaded {} instances", instances.len());
    Ok(instances)
}

/// Compares two Minecraft version strings, ordering older versions first.
fn compare_minecraft_versions(a: &str, b: &str) -> Ordering {
    compare_version_keys(&parse_version_key(a), &parse_version_key(b))
}

/// Parsed representation of a Minecraft version string used for ordering.
#[derive(Clone, Debug, PartialEq, Eq)]
enum VersionKey {
    /// Dated snapshot, e.g. "24w14a" or "25w14craftmine".
    Snapshot { year: u16, week: u8, letter: String },
    /// Regular release, optionally with a "pre" / "rc" suffix, e.g. "1.20.1".
    Releaseish {
        major: u8,
        minor: u8,
        patch: u8,
        prerelease: Option<(u8, u8)>,
    },
    /// Anything that could not be parsed.
    Unknown(String),
}

fn parse_version_key(raw: &str) -> VersionKey {
    parse_snapshot(raw)
        .or_else(|| parse_release(raw))
        .unwrap_or_else(|| VersionKey::Unknown(raw.to_string()))
}

/// Parses dated snapshots like "24w14a" (year, week, letter).
fn parse_snapshot(raw: &str) -> Option<VersionKey> {
    let bytes = raw.as_bytes();
    if bytes.len() < 5 || !bytes[..2].iter().all(u8::is_ascii_digit) || bytes[2] != b'w' {
        return None;
    }
    let year = raw[..2].parse().ok()?;
    let rest = &raw[3..];
    let week_digits_len = rest
        .as_bytes()
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if week_digits_len == 0 {
        return None;
    }
    let week = rest[..week_digits_len].parse().ok()?;
    let letter = rest[week_digits_len..].to_string();
    Some(VersionKey::Snapshot { year, week, letter })
}

/// Parses releases like "1.20.1", "1.20.1-pre1" or "1.21-rc3".
fn parse_release(raw: &str) -> Option<VersionKey> {
    let (version_part, prerelease) = match raw.split_once('-') {
        Some((version, suffix)) => (version, parse_prerelease(suffix)),
        None => (raw, None),
    };
    let mut segments = version_part.split('.');
    let major = segments.next()?.parse().ok()?;
    let minor = segments.next()?.parse().ok()?;
    let patch = segments.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    Some(VersionKey::Releaseish {
        major,
        minor,
        patch,
        prerelease,
    })
}

/// Parses a "preN" / "rcN" suffix into `(stage, index)` where pre = 0, rc = 1.
fn parse_prerelease(suffix: &str) -> Option<(u8, u8)> {
    let (stage, rest) = suffix
        .strip_prefix("pre")
        .map(|rest| (0u8, rest))
        .or_else(|| suffix.strip_prefix("rc").map(|rest| (1u8, rest)))?;
    Some((stage, rest.parse().ok()?))
}

fn compare_version_keys(a: &VersionKey, b: &VersionKey) -> Ordering {
    match (a, b) {
        (
            VersionKey::Snapshot {
                year: ay,
                week: aw,
                letter: al,
            },
            VersionKey::Snapshot {
                year: by,
                week: bw,
                letter: bl,
            },
        ) => (ay, aw, al).cmp(&(by, bw, bl)),
        (VersionKey::Releaseish { .. }, VersionKey::Releaseish { .. }) => compare_releaseish(a, b),
        (VersionKey::Snapshot { .. }, VersionKey::Releaseish { .. }) => {
            compare_snapshot_to_release(a, b)
        }
        (VersionKey::Releaseish { .. }, VersionKey::Snapshot { .. }) => {
            compare_snapshot_to_release(b, a).reverse()
        }
        (VersionKey::Unknown(x), VersionKey::Unknown(y)) => x.cmp(y),
        (VersionKey::Unknown(_), _) => Ordering::Less,
        (_, VersionKey::Unknown(_)) => Ordering::Greater,
    }
}

fn compare_releaseish(a: &VersionKey, b: &VersionKey) -> Ordering {
    match (a, b) {
        (
            VersionKey::Releaseish {
                major: am,
                minor: amin,
                patch: ap,
                prerelease: apr,
            },
            VersionKey::Releaseish {
                major: bm,
                minor: bmin,
                patch: bp,
                prerelease: bpr,
            },
        ) => (am, amin, ap)
            .cmp(&(bm, bmin, bp))
            .then_with(|| compare_prerelease(apr, bpr)),
        _ => unreachable!("compared a non-release key as a release"),
    }
}

/// Compares optional prerelease stages. A plain release (None) is newer than any
/// prerelease; within prereleases "rc" is newer than "pre", then the index.
fn compare_prerelease(a: &Option<(u8, u8)>, b: &Option<(u8, u8)>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    }
}

/// Places a dated snapshot relative to a release by comparing dates.
fn compare_snapshot_to_release(snapshot: &VersionKey, release: &VersionKey) -> Ordering {
    let (VersionKey::Snapshot { year, week, .. }, VersionKey::Releaseish { minor, patch, .. }) =
        (snapshot, release)
    else {
        unreachable!("compare_snapshot_to_release requires a snapshot and a release")
    };
    let snapshot_date = (2000 + *year, *week);
    let release_date = (release_year(*minor), release_week(*patch));
    snapshot_date.cmp(&release_date)
}

/// Approximate year in which the first release of a given minor version shipped.
fn release_year(minor: u8) -> u16 {
    match minor {
        16 => 2020,
        17 | 18 => 2021,
        19 => 2022,
        20 => 2023,
        21 => 2024,
        22 => 2025,
        minor => 2000 + u16::from(minor) + 3,
    }
}

/// Rough week-of-year a release with the given patch number shipped.
fn release_week(patch: u8) -> u8 {
    (24 + patch * 8).min(52)
}

/// Reads a single instance by id.
pub fn get_instance_by_id(id: &str) -> Option<Instance> {
    let instance_root = DATA_LOCATION.get_instance_root(id);
    let config_file = instance_root.join("instance.toml");
    if let Ok(config_content) = std::fs::read_to_string(config_file)
        && let Ok(config) = toml::from_str::<InstanceConfig>(&config_content)
    {
        Some(Instance {
            config,
            installed: instance_root.join(".install.lock").is_file(),
            id: id.to_string(),
            last_played: get_launch_script_timestamp(id),
            has_background: instance_root.join("background").is_file(),
        })
    } else {
        None
    }
}

/// Updates the configuration file of an existing instance.
pub fn update_instance(config: InstanceConfig, id: &str) -> Result<()> {
    let instance_root = DATA_LOCATION.get_instance_root(id);
    let config_file = instance_root.join("instance.toml");
    std::fs::write(config_file, toml::to_string_pretty(&config)?)?;
    info!("Updated instance: {}", config.name);
    Ok(())
}

/// Deletes the instance directory corresponding to the given id.
pub fn delete_instance(id: &str) -> Result<()> {
    std::fs::remove_dir_all(DATA_LOCATION.get_instance_root(id))?;
    info!("Deleted {id}");
    Ok(())
}

/// Removes the `.install.lock` marker file of an instance.
pub fn remove_install_lock(id: &str) -> Result<()> {
    let lock_file = DATA_LOCATION.get_instance_root(id).join(".install.lock");
    if let Err(err) = std::fs::remove_file(lock_file)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        return Err(err.into());
    }
    Ok(())
}

/// The path of an instance's background image.
pub fn get_background_path(id: &str) -> PathBuf {
    DATA_LOCATION.get_instance_root(id).join("background")
}

/// Copies `path` over the instance's background image.
pub fn add_background_image(path: &std::path::Path, id: &str) -> Result<()> {
    std::fs::copy(path, get_background_path(id))?;
    Ok(())
}

/// Removes an instance's background image.
pub fn remove_background(id: &str) -> Result<()> {
    std::fs::remove_file(get_background_path(id))?;
    Ok(())
}

/// Represents a game instance, including its configuration, installation status
/// and unique id.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Instance {
    /// The configuration of the instance.
    pub config: InstanceConfig,
    /// Whether the instance has been installed.
    pub installed: bool,
    /// Unique identifier of the instance.
    pub id: String,
    pub last_played: Option<u64>,
    pub has_background: bool,
}

impl Instance {
    /// The version id used as the game directory, including the loader prefix.
    pub fn get_version_id(&self) -> Result<String> {
        let config = &self.config;
        config
            .runtime
            .mod_loader_type
            .as_ref()
            .map(|mod_loader_type| {
                let mod_loader_version = config
                    .runtime
                    .mod_loader_version
                    .as_ref()
                    .ok_or(Error::InvalidInstanceConfig)?;
                let minecraft_version = &config.runtime.minecraft;
                Ok(match mod_loader_type {
                    ModLoaderType::Fabric => {
                        format!("fabric-loader-{mod_loader_version}-{minecraft_version}")
                    }
                    ModLoaderType::Quilt => {
                        format!("quilt-loader-{mod_loader_version}-{minecraft_version}")
                    }
                    ModLoaderType::Forge => {
                        format!("{minecraft_version}-forge-{mod_loader_version}")
                    }
                    ModLoaderType::Neoforge => {
                        format!("neoforge-{mod_loader_version}")
                    }
                })
            })
            .unwrap_or(Ok(config.runtime.minecraft.clone()))
    }

    /// Whether the instance is marked as a favorite.
    pub fn is_starred(&self) -> bool {
        self.config
            .group
            .as_ref()
            .is_some_and(|groups| groups.iter().any(|group| group == "starred"))
    }
}

/// Total play time of an instance in seconds, parsed from its game logs.
pub fn calculate_playtime(instance_id: &str) -> Result<u64> {
    let instance_root = DATA_LOCATION.get_instance_root(instance_id);
    let logs_root = instance_root.join("logs");
    let total_play_time: u64 = std::fs::read_dir(logs_root)?
        .filter_map(|entry| {
            let entry = match entry {
                Err(_) => return None,
                Ok(entry) => entry,
            };
            let path = entry.path();
            if path.is_file() { Some(path) } else { None }
        })
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|path| match try_read_flate2_log_dir_entry(path) {
            Err(_) => 0,
            Ok(x) => x.unwrap_or_default(),
        })
        .sum();
    Ok(total_play_time)
}

fn try_read_flate2_log_dir_entry(path: PathBuf) -> Result<Option<u64>> {
    if let Some(file_name) = path.file_name()
        && file_name == "latest.log"
    {
        return try_read_nogz_log_dir_entry(path);
    }
    let file = std::fs::File::open(&path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);
    let mut first_time: Option<u64> = None;
    let mut last_time: Option<u64> = None;

    for line in reader.lines() {
        let line = line?;
        let Some(time) = parse_log_time(&line) else {
            continue;
        };
        match first_time {
            None => {
                first_time = Some(time);
            }
            Some(first) if time > first => {
                last_time = Some(time);
            }
            _ => {}
        }
    }
    Ok(first_time.zip(last_time).map(|(start, end)| end - start))
}

fn try_read_nogz_log_dir_entry(path: PathBuf) -> Result<Option<u64>> {
    let file = std::fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut first_time: Option<u64> = None;
    let mut last_time: Option<u64> = None;

    for line in reader.lines() {
        let line = line?;
        let Some(time) = parse_log_time(&line) else {
            continue;
        };
        match first_time {
            None => {
                first_time = Some(time);
            }
            Some(first) if time > first => {
                last_time = Some(time);
            }
            _ => {}
        }
    }
    Ok(first_time.zip(last_time).map(|(start, end)| end - start))
}

fn parse_log_time(line: &str) -> Option<u64> {
    let bytes = line.as_bytes();
    if bytes.len() < 10
        || bytes[0] != b'['
        || bytes[3] != b':'
        || bytes[6] != b':'
        || bytes[9] != b']'
    {
        return None;
    }
    let digits = [bytes[1], bytes[2], bytes[4], bytes[5], bytes[7], bytes[8]];
    if digits.iter().any(|c| !c.is_ascii_digit()) {
        return None;
    }
    let hour = (bytes[1] - b'0') as u64 * 10 + (bytes[2] - b'0') as u64;
    let minute = (bytes[4] - b'0') as u64 * 10 + (bytes[5] - b'0') as u64;
    let second = (bytes[7] - b'0') as u64 * 10 + (bytes[8] - b'0') as u64;
    if hour >= 24 || minute >= 60 || second >= 60 {
        return None;
    }
    Some(hour * 3600 + minute * 60 + second)
}

fn get_launch_script_timestamp(instance_id: &str) -> Option<u64> {
    #[cfg(not(target_os = "windows"))]
    let script_name = "conic-launch.sh";
    #[cfg(target_os = "windows")]
    let script_name = "conic-launch.bat";
    let script_path = DATA_LOCATION
        .get_instance_root(instance_id)
        .join(".cache")
        .join(script_name);
    let file = std::fs::File::open(script_path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().take(10) {
        let line = line.ok()?;
        let Some(timestamp) = line.split("created this file at ").nth(1) else {
            continue;
        };
        let timestamp = timestamp.trim_end_matches('.');
        if let Ok(timestamp) = timestamp.parse() {
            return Some(timestamp);
        }
    }
    None
}
