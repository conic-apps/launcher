// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::io::Read;

use serde_json::Value;
use sha2::Digest;

use download::{Checksum, DownloadTask, DownloadTaskType, progress::DownloadState};
use shared::HTTP_CLIENT;
use storage::LOCATIONS;

use crate::error::*;

pub async fn ensure_latest(progress: &DownloadState) -> Result<()> {
    log::info!("Checking that the authlib-injector is up to date");
    let path = LOCATIONS.launcher.authlib_injector.clone();
    let latest_version = HTTP_CLIENT
        .get("https://authlib-injector.yushi.moe/artifact/latest.json")
        .send()
        .await?
        .json::<Value>()
        .await?;
    let url = latest_version["download_url"]
        .as_str()
        .ok_or(Error::InvalidAuthlibResponse)?;
    // The published hash is nested under `checksums`, not a top-level field.
    let sha256 = latest_version["checksums"]["sha256"]
        .as_str()
        .ok_or(Error::InvalidAuthlibResponse)?;
    let published = latest_version["version"].as_str().unwrap_or("<unknown>");
    // A missing or unreadable injector means it has to be downloaded, not that
    // the launch is broken — this is the only code in the workspace that writes
    // the file, so treating absence as an error meant a fresh install could never
    // launch against a Yggdrasil server at all.
    let current = std::fs::File::open(&path)
        .ok()
        .and_then(|mut file| verify_sha256_from_read(&mut file, sha256));
    match current {
        Some(true) => {
            log::debug!("the authlib-injector {published} is already current");
            return Ok(());
        }
        Some(false) => log::info!(
            "the authlib-injector at {} is stale (published {published}); downloading",
            path.display()
        ),
        None => log::info!(
            "the authlib-injector is not installed at {} (published {published}); downloading",
            path.display()
        ),
    }
    log::info!("Downloading the authlib-injector from {url}");
    let download_task = DownloadTask {
        url: url.to_string(),
        file: path,
        size_bytes: None,
        checksum: Checksum::Sha256(sha256.to_string()),
        task_type: DownloadTaskType::AuthlibInjector,
    };
    download::download(&download_task, progress).await?;
    log::info!("the authlib-injector {published} is installed");
    Ok(())
}

fn verify_sha256_from_read<R: Read>(source: &mut R, checksum: &str) -> Option<bool> {
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = source.read(&mut buffer).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Some(format!("{:02x}", hasher.finalize()) == checksum)
}
