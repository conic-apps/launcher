// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CRUD for game instances: the data model and the filesystem logic.
//!
//! The app calls these functions directly and owns the UI state itself.
//!
//! The CRUD path stays `async` and on `tokio::fs`: listing instances reads and
//! parses one `instance.toml` per instance, which has no business running on
//! the thread that draws.

use std::{
    cmp::Ordering,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use flate2::read::GzDecoder;
use log::info;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use storage::LOCATIONS;
use uuid::Uuid;

mod config;
mod error;

// `crate::` on both: this crate root declares a `config` module *and* depends
// on the `config` crate, so a bare `config::` is ambiguous here (E0659).
pub use crate::config::*;
pub use crate::error::*;

/// Creates a new game instance using the provided configuration.
pub async fn create_instance(config: InstanceConfig, id: Option<&str>) -> Result<String> {
    let random_uuid = Uuid::new_v4().to_string();
    let id = id.unwrap_or(&random_uuid);
    let instance_root = LOCATIONS.instances.get_instance_root(id);
    let config_file_path = instance_root.join("instance.toml");
    if let Some(parent) = config_file_path.parent() {
        tokio::fs::create_dir_all(parent).await?
    }
    tokio::fs::write(config_file_path, toml::to_string_pretty(&config)?).await?;
    info!("Created instance: {}", config.name);
    Ok(id.to_string())
}

/// Sorting strategies for listing instances.
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
pub async fn list_instances(sort_by: SortBy) -> Result<Vec<Instance>> {
    let instances_folder = &LOCATIONS.instances.root;
    tokio::fs::create_dir_all(instances_folder).await?;
    let mut folder_entries = tokio::fs::read_dir(instances_folder).await?;
    let mut instances = Vec::new();

    while let Some(entry) = folder_entries.next_entry().await? {
        let file_type = match entry.file_type().await {
            Err(_) => continue,
            Ok(file_type) => file_type,
        };
        if !file_type.is_dir() {
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
        let config_content = match tokio::fs::read_to_string(instance_config).await {
            Err(_) => continue,
            Ok(content) => content,
        };
        let instance_id = folder_name;
        let instance = Instance {
            config: match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => config,
                Err(_) => continue,
            },
            installed: tokio::fs::metadata(path.join(".install.lock"))
                .await
                .is_ok(),
            last_played: get_launch_script_timestamp(&instance_id),
            last_exit_abnormal: last_exit_abnormal(&instance_id),
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
pub async fn get_instance_by_id(id: &str) -> Option<Instance> {
    let instance_root = LOCATIONS.instances.get_instance_root(id);
    let config_file = instance_root.join("instance.toml");
    if let Ok(config_content) = tokio::fs::read_to_string(config_file).await
        && let Ok(config) = toml::from_str::<InstanceConfig>(&config_content)
    {
        Some(Instance {
            config,
            installed: tokio::fs::metadata(instance_root.join(".install.lock"))
                .await
                .is_ok(),
            last_played: get_launch_script_timestamp(id),
            last_exit_abnormal: last_exit_abnormal(id),
            id: id.to_string(),
            has_background: instance_root.join("background").is_file(),
        })
    } else {
        None
    }
}

/// Updates the configuration file of an existing instance.
pub async fn update_instance(config: InstanceConfig, id: &str) -> Result<()> {
    let instance_root = LOCATIONS.instances.get_instance_root(id);
    let config_file = instance_root.join("instance.toml");
    tokio::fs::write(config_file, toml::to_string_pretty(&config)?).await?;
    info!("Updated instance: {}", config.name);
    Ok(())
}

/// Deletes the instance directory corresponding to the given id.
pub async fn delete_instance(id: &str) -> Result<()> {
    tokio::fs::remove_dir_all(LOCATIONS.instances.get_instance_root(id)).await?;
    info!("Deleted {id}");
    Ok(())
}

/// Removes the `.install.lock` marker file of an instance.
pub async fn remove_install_lock(id: &str) -> Result<()> {
    let lock_file = LOCATIONS
        .instances
        .get_instance_root(id)
        .join(".install.lock");
    if let Err(err) = tokio::fs::remove_file(lock_file).await
        && err.kind() != std::io::ErrorKind::NotFound
    {
        return Err(err.into());
    }
    Ok(())
}

/// The marker file whose presence means the instance's last run exited
/// abnormally.
///
/// A file rather than a field in `instance.toml`, for the same reason
/// `.install.lock` is one: it is runtime state, not user configuration, so a
/// hand-edited config cannot clear it and an interrupted run cannot corrupt it.
fn crash_marker(id: &str) -> PathBuf {
    LOCATIONS
        .instances
        .get_instance_root(id)
        .join(".last-exit-crash")
}

/// Records that the instance's last run exited abnormally.
pub fn mark_last_exit_abnormal(id: &str) -> Result<()> {
    std::fs::write(crash_marker(id), b"")?;
    Ok(())
}

/// Clears the abnormal-exit marker, after a clean exit.
///
/// Idempotent: an instance with no marker is already "not crashed".
pub fn clear_last_exit_abnormal(id: &str) -> Result<()> {
    match std::fs::remove_file(crash_marker(id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Whether the instance's last run exited abnormally.
pub fn last_exit_abnormal(id: &str) -> bool {
    crash_marker(id).is_file()
}

/// The path of an instance's background image.
pub fn get_background_path(id: &str) -> PathBuf {
    LOCATIONS.instances.get_instance_root(id).join("background")
}

/// Copies `path` over the instance's background image.
pub async fn add_background_image(path: &std::path::Path, id: &str) -> Result<()> {
    let dest = get_background_path(id);
    tokio::fs::copy(path, &dest).await?;
    stamp_now(&dest);
    Ok(())
}

/// Sets `path`'s modified time to now.
///
/// `fs::copy` carries the *source* file's timestamp over on macOS, so replacing
/// the background with an image that shares its mtime leaves the file looking
/// unchanged — and the background loader tells a replacement from a re-read by
/// that timestamp, so the new picture would not appear until a restart. Stamping
/// now makes every replacement a distinct revision.
fn stamp_now(path: &std::path::Path) {
    if let Err(error) = std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(std::time::SystemTime::now()))
    {
        // Not fatal: the picture is in place, only its freshness cannot be told.
        log::debug!("could not stamp '{}': {error}", path.display());
    }
}

/// Removes an instance's background image.
pub async fn remove_background(id: &str) -> Result<()> {
    tokio::fs::remove_file(get_background_path(id)).await?;
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
    /// Whether the instance's last run exited abnormally. Derived from a marker
    /// file (see [`mark_last_exit_abnormal`]), not persisted in `instance.toml`.
    #[serde(default)]
    pub last_exit_abnormal: bool,
}

impl Instance {
    /// This instance's version id, including the loader prefix, which names the
    /// version directory under `versions/`.
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
    let instance_root = LOCATIONS.instances.get_instance_root(instance_id);
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
    let script_path = LOCATIONS
        .instances
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

#[cfg(test)]
mod tests {
    use super::*;

    fn order(a: &str, b: &str) -> Ordering {
        compare_minecraft_versions(a, b)
    }

    #[test]
    fn releases_are_ordered_by_major_minor_patch() {
        assert_eq!(order("1.20.1", "1.20.2"), Ordering::Less);
        assert_eq!(order("1.20.10", "1.21"), Ordering::Less);
        assert_eq!(order("1.7.10", "1.8"), Ordering::Less);
        assert_eq!(order("1.20.1", "1.20.1"), Ordering::Equal);
    }

    #[test]
    fn a_prerelease_sorts_before_its_release() {
        assert_eq!(order("1.20.1-pre1", "1.20.1"), Ordering::Less);
        // rc is newer than pre.
        assert_eq!(order("1.20.1-pre2", "1.20.1-rc1"), Ordering::Less);
        assert_eq!(order("1.20.1-rc1", "1.20.1"), Ordering::Less);
    }

    #[test]
    fn snapshots_are_ordered_by_year_week_and_letter() {
        assert_eq!(order("24w14a", "24w15a"), Ordering::Less);
        assert_eq!(order("24w14a", "24w14b"), Ordering::Less);
        assert_eq!(order("25w01a", "24w50a"), Ordering::Greater);
    }

    #[test]
    fn a_snapshot_sits_between_the_releases_around_it() {
        // 24w14a shipped in early 2024, after 1.20 (2023) and before 1.21.
        assert_eq!(order("24w14a", "1.20"), Ordering::Greater);
        assert_eq!(order("24w14a", "1.21"), Ordering::Less);
    }

    #[test]
    fn an_unparseable_version_sorts_before_a_known_one() {
        assert_eq!(order("not-a-version", "1.20.1"), Ordering::Less);
        assert_eq!(order("1.20.1", "not-a-version"), Ordering::Greater);
        assert_eq!(order("alpha", "beta"), Ordering::Less);
    }

    #[test]
    fn the_version_parsers_split_the_expected_pieces() {
        assert_eq!(
            parse_snapshot("24w14a"),
            Some(VersionKey::Snapshot {
                year: 24,
                week: 14,
                letter: "a".to_string()
            })
        );
        assert_eq!(
            parse_release("1.21.4"),
            Some(VersionKey::Releaseish {
                major: 1,
                minor: 21,
                patch: 4,
                prerelease: None
            })
        );
        assert_eq!(
            parse_release("1.21-rc2"),
            Some(VersionKey::Releaseish {
                major: 1,
                minor: 21,
                patch: 0,
                prerelease: Some((1, 2))
            })
        );
        assert_eq!(parse_prerelease("pre3"), Some((0, 3)));
        assert_eq!(parse_prerelease("rc10"), Some((1, 10)));
        assert_eq!(parse_prerelease("beta1"), None);
    }

    #[test]
    fn a_log_timestamp_is_seconds_since_midnight() {
        assert_eq!(
            parse_log_time("[12:34:56] [Client thread/INFO]"),
            Some(45296)
        );
        assert_eq!(parse_log_time("[00:00:00]"), Some(0));
        assert_eq!(parse_log_time("[23:59:59]"), Some(86399));
    }

    #[test]
    fn something_that_is_not_a_log_timestamp_is_ignored() {
        assert_eq!(parse_log_time("12:34:56 no brackets"), None);
        assert_eq!(parse_log_time("[24:00:00] hour out of range"), None);
        assert_eq!(parse_log_time("[12:60:00] minute out of range"), None);
        assert_eq!(parse_log_time("[ab:cd:ef]"), None);
        assert_eq!(parse_log_time(""), None);
    }

    #[test]
    fn the_version_id_carries_the_loader_prefix() {
        let instance = |loader, version: Option<&str>| {
            let mut config = InstanceConfig::new("Test", "1.20.1");
            config.runtime.mod_loader_type = loader;
            config.runtime.mod_loader_version = version.map(ToString::to_string);
            Instance {
                config,
                ..Default::default()
            }
        };

        assert_eq!(
            instance(Some(ModLoaderType::Fabric), Some("0.15.0"))
                .get_version_id()
                .unwrap(),
            "fabric-loader-0.15.0-1.20.1"
        );
        assert_eq!(
            instance(Some(ModLoaderType::Quilt), Some("0.20.0"))
                .get_version_id()
                .unwrap(),
            "quilt-loader-0.20.0-1.20.1"
        );
        assert_eq!(
            instance(Some(ModLoaderType::Forge), Some("47.2.0"))
                .get_version_id()
                .unwrap(),
            "1.20.1-forge-47.2.0"
        );
        assert_eq!(
            instance(Some(ModLoaderType::Neoforge), Some("21.1.0"))
                .get_version_id()
                .unwrap(),
            "neoforge-21.1.0"
        );
        assert_eq!(instance(None, None).get_version_id().unwrap(), "1.20.1");
    }

    #[test]
    fn a_loader_without_a_version_is_an_error() {
        let mut config = InstanceConfig::new("Test", "1.20.1");
        config.runtime.mod_loader_type = Some(ModLoaderType::Fabric);
        let instance = Instance {
            config,
            ..Default::default()
        };
        assert!(instance.get_version_id().is_err());
    }

    #[test]
    fn starred_is_a_membership_test_on_the_groups() {
        let with_groups = |groups: Option<Vec<String>>| {
            let mut config = InstanceConfig::new("Test", "1.20.1");
            config.group = groups;
            Instance {
                config,
                ..Default::default()
            }
        };
        assert!(with_groups(Some(vec!["starred".to_string()])).is_starred());
        assert!(!with_groups(Some(vec!["favourites".to_string()])).is_starred());
        assert!(!with_groups(None).is_starred());
    }
}
