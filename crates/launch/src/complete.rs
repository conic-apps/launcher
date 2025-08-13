// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::str::FromStr;

use log::{info, warn};

use download::{DownloadTask, download_and_check, filter_existing_and_verified_files};
use folder::{DATA_LOCATION, MinecraftLocation};
use install::vanilla::{generate_assets_downloads, generate_libraries_downloads};
use instance::Instance;
use task::Progress;
use version::Version;

/// Completes and verifies all assets and libraries files for the given instance and Minecraft location.
///
/// This function checks if lock files exist to skip redundant verification. If lock files are missing,
/// it will verify and download missing or corrupted assets and libraries, then create the lock files.
/// > NOTE: If game crashed, the lock file should be delete!
/// > TODO: Write create date in lock file, auto delete when passing 10 days
///
/// # Arguments
///
/// * `instance` - The Minecraft instance whose files to verify.
/// * `minecraft_location` - The Minecraft location to resolve file paths.
pub async fn complete_files(instance: &Instance, minecraft_location: &MinecraftLocation) {
    // TODO: Parallel
    let assets_lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".conic-assets-ok");
    let libraries_lock_file = DATA_LOCATION
        .get_instance_root(&instance.id)
        .join(".conic-libraries-ok");
    if std::fs::metadata(&assets_lock_file).is_ok() {
        info!("Found file \".conic-assets-ok\", no need to check assets files.");
    } else {
        info!("Checking and completing assets files");
        complete_assets_files(instance, minecraft_location).await;
        info!("Saving assets lock file");
        std::fs::write(assets_lock_file, "ok").unwrap();
    }
    if std::fs::metadata(&libraries_lock_file).is_ok() {
        info!("Found file \".conic-libraries-ok\", no need to check libraries files.");
    } else {
        info!("Checking and completing libraries files");
        complete_libraries_files(instance, minecraft_location).await;
        info!("Saving libraries lock file");
        std::fs::write(libraries_lock_file, "ok").unwrap();
    }
}

/// Completes missing or corrupted asset files for the given instance.
async fn complete_assets_files(instance: &Instance, minecraft_location: &MinecraftLocation) {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id());
    let raw_version_json = async_fs::read_to_string(version_json_path).await.unwrap();
    let resolved_version = Version::from_str(&raw_version_json)
        .unwrap()
        .resolve(minecraft_location, &[])
        .await
        .unwrap();

    let assets_downloads = generate_assets_downloads(minecraft_location, &resolved_version)
        .await
        .unwrap();
    let progress = Progress::default(); // TODO: send it to frontend
    let downloads = filter_existing_and_verified_files(assets_downloads, &progress);
    if !downloads.is_empty() {
        download_files(downloads).await.unwrap(); // TODO: use download module
    }
}

/// Completes missing or corrupted library files for the given instance.
async fn complete_libraries_files(instance: &Instance, minecraft_location: &MinecraftLocation) {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id());
    let raw_version_json = async_fs::read_to_string(version_json_path).await.unwrap();
    let resolved_version = Version::from_str(&raw_version_json)
        .unwrap()
        .resolve(minecraft_location, &[])
        .await
        .unwrap();

    let library_downloads = generate_libraries_downloads(minecraft_location, &resolved_version);
    let progress = Progress::default(); // TODO: send it to frontend
    let downloads = filter_existing_and_verified_files(library_downloads, &progress);
    if !downloads.is_empty() {
        download_files(downloads).await.unwrap(); // TODO: use download module
    }
}

// TODO: Remove this
async fn download_files(downloads: Vec<DownloadTask>) -> anyhow::Result<()> {
    for download in downloads {
        let mut retried = 0;
        while retried <= 5 {
            retried += 1;
            let progress = Progress::default();
            match download_and_check(&download, &progress).await {
                Ok(_) => break,
                Err(_) => warn!("Download failed: {}, retried: {}", &download.url, retried),
            }
        }
    }
    Ok(())
}
