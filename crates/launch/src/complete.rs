// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use log::{info, warn};

use config::download::DownloadConfig;
use download::progress::DownloadState;
use install::vanilla::{generate_assets_downloads, generate_libraries_downloads};
use instance::Instance;
use storage::{LOCATIONS, MinecraftLocation};
use version::{Version, resolve_version};

use crate::error::*;

/// Completes and verifies all assets, libraries and Mojang-provided Java
/// runtime files for the given instance.
///
/// A lock file is honoured until its TTL; when one is missing or stale the
/// corresponding set is re-verified and the lock rewritten. An abnormal last
/// run (its crash marker set) deletes the locks first, forcing one full
/// re-check, then clears the marker.
pub async fn complete_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: DownloadState,
    prefer_mojang_java: bool,
    config: &DownloadConfig,
) -> Result<()> {
    let instance_root = LOCATIONS.instances.get_instance_root(&instance.id);
    // An abnormal run may have left half-written files, so the locks must not
    // vouch for them: the crash marker forces one full re-check, then clears.
    // Mirrors HMCL's `unmarkLaunchedAbnormally`.
    if instance::last_exit_abnormal(&instance.id) {
        info!(
            "The previous run of instance {} exited abnormally; re-checking files",
            instance.id
        );
        for lock in [
            ".conic-assets-ok",
            ".conic-libraries-ok",
            ".java-runtime-ok",
        ] {
            let _ = std::fs::remove_file(instance_root.join(lock));
        }
        if let Err(error) = instance::clear_last_exit_abnormal(&instance.id) {
            warn!(
                "Could not clear the crash marker of instance {}: {error}",
                instance.id
            );
        }
    }
    let assets_lock_file = instance_root.join(".conic-assets-ok");
    let libraries_lock_file = instance_root.join(".conic-libraries-ok");
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
        if let Err(error) = save_lock_file(&assets_lock_file) {
            // If this save is lost, every launch re-verifies the whole assets
            // set again, silently.
            warn!(
                "Could not write the assets lock file {}: {error}",
                assets_lock_file.display()
            );
        }
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
        if let Err(error) = save_lock_file(&libraries_lock_file) {
            // If this save is lost, every launch re-verifies the whole
            // libraries set again, silently.
            warn!(
                "Could not write the libraries lock file {}: {error}",
                libraries_lock_file.display()
            );
        }
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

async fn complete_assets_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id()?);
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await?;
    let resolved_version = resolve_version(
        &Version::from_str(&raw_version_json)?,
        minecraft_location,
        &[],
    )?;
    if let Some(asset_index) = resolved_version.asset_index {
        let assets_downloads = generate_assets_downloads(minecraft_location, &asset_index).await?;
        download::download_concurrent(assets_downloads, &progress, config).await?;
    };
    Ok(())
}

async fn complete_libraries_files(
    instance: &Instance,
    minecraft_location: &MinecraftLocation,
    progress: DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id()?);
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await?;
    let resolved_version = resolve_version(
        &Version::from_str(&raw_version_json)?,
        minecraft_location,
        &[],
    )?;
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
    let lock_file = LOCATIONS
        .instances
        .get_instance_root(&instance.id)
        .join(".java-runtime-ok");
    if try_load_lock_file(&lock_file).is_some() {
        info!("Found file \".java-runtime-ok\", no need to check Java runtime files.");
        return Ok(());
    }
    info!("Checking and completing Mojang-provided Java runtime");
    install::java::install_for_instance(instance, progress, config).await?;
    info!("Saving Java runtime lock file");
    if let Err(error) = save_lock_file(&lock_file) {
        warn!(
            "Could not write the Java runtime lock file {}: {error}",
            lock_file.display()
        );
    }
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
