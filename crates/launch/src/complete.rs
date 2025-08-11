// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{io::Read, str::FromStr};

use anyhow::anyhow;
use log::{info, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use shared::HTTP_CLIENT;
use tokio::io::AsyncWriteExt;

use download::DownloadTask;
use folder::{DATA_LOCATION, MinecraftLocation};
use install::vanilla::{generate_assets_downloads, generate_libraries_downloads};
use instance::Instance;
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
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await.unwrap();
    let resolved_version = Version::from_str(&raw_version_json)
        .unwrap()
        .resolve(minecraft_location, &[])
        .await
        .unwrap();

    let assets_downloads = generate_assets_downloads(minecraft_location, &resolved_version)
        .await
        .unwrap();
    let downloads = filter_correct_files(assets_downloads).await;
    if !downloads.is_empty() {
        download_files(downloads).await.unwrap();
    }
}

/// Completes missing or corrupted library files for the given instance.
async fn complete_libraries_files(instance: &Instance, minecraft_location: &MinecraftLocation) {
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id());
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await.unwrap();
    let resolved_version = Version::from_str(&raw_version_json)
        .unwrap()
        .resolve(minecraft_location, &[])
        .await
        .unwrap();

    let library_downloads = generate_libraries_downloads(minecraft_location, &resolved_version);
    let downloads = filter_correct_files(library_downloads).await;
    if !downloads.is_empty() {
        download_files(downloads).await.unwrap();
    }
}

/// Filters out downloads that already exist and match the expected SHA1 hash.
///
/// Uses parallel iteration for efficiency.
///
/// # Arguments
///
/// * `downloads` - A vector of Download structs representing files to verify.
///
/// # Returns
///
/// A vector of Download structs that need to be downloaded or re-downloaded.
pub async fn filter_correct_files(downloads: Vec<DownloadTask>) -> Vec<DownloadTask> {
    downloads
        .into_par_iter()
        .filter(|download| {
            if std::fs::metadata(&download.file).is_err() {
                return true;
            }
            let mut file = match std::fs::File::open(&download.file) {
                Ok(file) => file,
                Err(_) => {
                    return true;
                }
            };
            if download.sha1.is_none() {
                return true;
            };
            let file_hash = calculate_sha1_from_read(&mut file);
            &file_hash != download.sha1.as_ref().unwrap()
        })
        .collect()
}

/// Calculates the SHA1 hash string from a readable source.
///
/// # Arguments
///
/// * `source` - A mutable reference to an object implementing `Read`.
///
/// # Returns
///
/// A hexadecimal SHA1 hash string.
fn calculate_sha1_from_read<R: Read>(source: &mut R) -> String {
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = source.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    hasher.digest().to_string()
}

/// Downloads the list of files, retrying each up to 5 times on failure.
///
/// > NOTE: This function do NOT support parallel download
///
/// # Arguments
///
/// * `downloads` - Vector of Download structs to download.
///
/// # Errors
///
/// Returns an error if any download fails after retries.
async fn download_files(downloads: Vec<DownloadTask>) -> anyhow::Result<()> {
    for download in downloads {
        let mut retried = 0;
        while retried <= 5 {
            retried += 1;
            match download_and_check(&download).await {
                Ok(_) => break,
                Err(_) => warn!("Download failed: {}, retried: {}", &download.url, retried),
            }
        }
    }
    Ok(())
}

/// Downloads a single file and checks its SHA1 hash if available.
///
/// # Arguments
///
/// * `download` - The Download struct containing URL, file path, and expected SHA1.
///
/// # Errors
///
/// Returns an error if download fails or SHA1 verification fails.
async fn download_and_check(download: &DownloadTask) -> anyhow::Result<()> {
    let file_path = download.file.clone();
    tokio::fs::create_dir_all(file_path.parent().ok_or(anyhow::Error::msg(
        "Unknown Error in instance/mod.rs".to_string(),
    ))?)
    .await?;
    let mut response = HTTP_CLIENT.get(download.url.clone()).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!("Download failed"));
    }
    let mut file = tokio::fs::File::create(&file_path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.sync_all().await?;
    drop(file);
    let mut file = std::fs::File::open(&file_path).unwrap();
    if let Some(sha1) = download.sha1.clone()
        && calculate_sha1_from_read(&mut file) != sha1
    {
        return Err(anyhow::Error::msg("SHA1 check failed".to_string()));
    }
    Ok(())
}
