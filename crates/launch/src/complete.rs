// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    path::PathBuf,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use config::download::DownloadConfig;
use log::info;

use download::task::Progress;
use folder::{DATA_LOCATION, MinecraftLocation};
use install::vanilla::{generate_assets_downloads, generate_libraries_downloads};
use instance::Instance;
use version::{Version, resolve_version};

use crate::error::*;

/// Completes and verifies all assets and libraries files for the given instance and Minecraft location.
///
/// This function checks if lock files exist to skip redundant verification. If lock files are missing,
/// it will verify and download missing or corrupted assets and libraries, then create the lock files.
/// > NOTE: If game crashed, the lock file should be delete!
///
/// # Arguments
///
/// * `instance` - The Minecraft instance whose files to verify.
/// * `minecraft_location` - The Minecraft location to resolve file paths.
pub async fn complete_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: Progress,
    config: &DownloadConfig,
) -> Result<()> {
    let assets_lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".conic-assets-ok");
    let libraries_lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".conic-libraries-ok");
    if try_load_lock_file(&assets_lock_file).await.is_some() {
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
        let _ = save_lock_file(&assets_lock_file).await;
    }
    if try_load_lock_file(&libraries_lock_file).await.is_some() {
        info!("Found file \".conic-libraries-ok\", no need to check libraries files.");
    } else {
        info!("Checking and completing libraries files");
        complete_libraries_files(instance, minecraft_location, progress, config.clone()).await?;
        info!("Saving libraries lock file");
        let _ = save_lock_file(&libraries_lock_file).await;
    }
    Ok(())
}

async fn try_load_lock_file(path: &PathBuf) -> Option<()> {
    let contents = async_fs::read_to_string(path)
        .await
        .ok()?
        .parse::<u64>()
        .ok()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect system time")
        .as_secs();
    if now - contents > 10 * 24 * 60 * 60 {
        return None;
    };
    Some(())
}

async fn save_lock_file(path: &PathBuf) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect system time")
        .as_secs();
    std::fs::write(path, now.to_string())?;
    Ok(())
}

/// Completes missing or corrupted asset files for the given instance.
async fn complete_assets_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: Progress,
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

    let assets_downloads = generate_assets_downloads(minecraft_location, &resolved_version).await?;
    download::download_concurrent(assets_downloads, &progress, config).await?;
    Ok(())
}

/// Completes missing or corrupted library files for the given instance.
async fn complete_libraries_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: Progress,
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

    let library_downloads = generate_libraries_downloads(minecraft_location, &resolved_version);
    download::download_concurrent(library_downloads, &progress, config).await?;
    Ok(())
}
