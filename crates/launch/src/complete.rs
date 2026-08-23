// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use config::download::DownloadConfig;
use log::{info, warn};

use download::progress::DownloadState;
use folder::{DATA_LOCATION, MinecraftLocation};
use install::vanilla::{generate_assets_downloads, generate_libraries_downloads};
use instance::Instance;
use version::{Version, resolve_version};

use crate::error::*;

/// Completes and verifies all assets, libraries and Mojang-provided Java
/// runtime files for the given instance and Minecraft location.
///
/// This function checks if lock files exist to skip redundant verification. If lock files are missing,
/// it will verify and download missing or corrupted assets, libraries and the
/// Mojang-provided Java runtime (when preferred), then create the lock files.
/// > NOTE: If game crashed, the lock file should be delete!
///
/// # Arguments
///
/// * `instance` - The Minecraft instance whose files to verify.
/// * `minecraft_location` - The Minecraft location to resolve file paths.
/// * `prefer_mojang_java` - Whether the Mojang-provided Java runtime is used.
pub async fn complete_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: DownloadState,
    prefer_mojang_java: bool,
    config: &DownloadConfig,
) -> Result<()> {
    let assets_lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".conic-assets-ok");
    let libraries_lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".conic-libraries-ok");
    if try_load_lock_file(&assets_lock_file).is_some() {
        info!("Found file \".conic-assets-ok\", no need to check assets files.");
    } else {
        info!("Checking and completing assets files");
        complete_assets_files(
            instance,
            minecraft_location,
            progress.clone(),
            config.clone(),
        )
        .await?;
        info!("Saving assets lock file");
        let _ = save_lock_file(&assets_lock_file);
    }
    if try_load_lock_file(&libraries_lock_file).is_some() {
        info!("Found file \".conic-libraries-ok\", no need to check libraries files.");
    } else {
        info!("Checking and completing libraries files");
        complete_libraries_files(
            instance,
            minecraft_location,
            progress.clone(),
            config.clone(),
        )
        .await?;
        info!("Saving libraries lock file");
        let _ = save_lock_file(&libraries_lock_file);
    }
    // Best-effort: a failure here must not abort the launch because the Java
    // resolution step either falls back to a system runtime or reports
    // `NoSuitableJavaRuntime` when nothing usable is available.
    if prefer_mojang_java
        && instance.config.launch_config.java_path.is_none()
        && let Err(error) = complete_java_runtime_files(instance, &progress, config.clone()).await
    {
        warn!("Failed to ensure the Mojang-provided Java runtime: {error}");
    }
    Ok(())
}

/// Completes missing or corrupted asset files for the given instance.
async fn complete_assets_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id()?);
    let raw_version_json = async_fs::read_to_string(version_json_path).await?;
    let resolved_version = resolve_version(
        &Version::from_str(&raw_version_json)?,
        minecraft_location,
        &[],
    )
    .await?;
    if let Some(asset_index) = resolved_version.asset_index {
        let assets_downloads = generate_assets_downloads(minecraft_location, &asset_index).await?;
        download::download_concurrent(assets_downloads, &progress, config).await?;
    };
    Ok(())
}

/// Completes missing or corrupted library files for the given instance.
async fn complete_libraries_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id()?);
    let raw_version_json = async_fs::read_to_string(version_json_path).await?;
    let resolved_version = resolve_version(
        &Version::from_str(&raw_version_json)?,
        minecraft_location,
        &[],
    )
    .await?;
    let library_downloads =
        generate_libraries_downloads(minecraft_location, &resolved_version.libraries);
    download::download_concurrent(library_downloads, &progress, config).await?;
    Ok(())
}

async fn complete_java_runtime_files(
    instance: &Instance,
    progress: &DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".java-runtime-ok");
    if try_load_lock_file(&lock_file).is_some() {
        info!("Found file \".java-runtime-ok\", no need to check Java runtime files.");
        return Ok(());
    }
    info!("Checking and completing Mojang-provided Java runtime");
    install::java::install_for_instance(instance, progress, config).await?;
    info!("Saving Java runtime lock file");
    let _ = save_lock_file(&lock_file);
    Ok(())
}

/// Time-to-live applied by [`try_load_lock_file`] before a lock file is
/// considered stale (10 days).
pub const LOCK_FILE_TTL_SECONDS: u64 = 10 * 24 * 60 * 60;

/// Reads a timestamped lock file written by [`save_lock_file`], returning `None`
/// when it is missing, unreadable or older than [`LOCK_FILE_TTL_SECONDS`].
pub fn try_load_lock_file(path: &Path) -> Option<()> {
    let contents = std::fs::read_to_string(path).ok()?.parse::<u64>().ok()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect system time")
        .as_secs();
    if now - contents > LOCK_FILE_TTL_SECONDS {
        return None;
    }
    Some(())
}

/// Writes a lock file containing the current unix timestamp.
pub fn save_lock_file(path: &Path) -> std::io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect system time")
        .as_secs();
    std::fs::write(path, now.to_string())
}
