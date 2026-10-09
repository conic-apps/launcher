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
    // This runs on every Yggdrasil launch and logged nothing at all, so a
    // launcher whose injector could not be fetched was silent right up to the
    // point where the game refused to start.
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
    let sha256 = latest_version["download_url"]
        .as_str()
        .ok_or(Error::InvalidAuthlibResponse)?;
    // A missing injector is the case that matters: this is the "it has never been
    // downloaded" path, and it aborted the launch with a bare `Io` before.
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            log::warn!(
                "The authlib-injector is not at {} ({error})",
                path.display()
            );
            return Err(error.into());
        }
    };
    // Read once: the handle is at EOF afterwards, so a second pass would hash
    // nothing.
    let matches = verify_sha256_from_read(&mut file, sha256);
    // The comparison below reads as inverted — it downloads when the hash already
    // matches — and the log now records which way it went, so a stale or missing
    // jar is visible either way. See the bug list: `sha256` is read from the
    // `download_url` key, and the real `sha256` field of the document is never read.
    match matches {
        Some(true) => log::info!("The authlib-injector matches the published hash"),
        Some(false) => log::warn!(
            "The authlib-injector at {} does not match the published hash; it is being \
             left as it is",
            path.display()
        ),
        None => log::warn!(
            "The authlib-injector at {} could not be read to verify it",
            path.display()
        ),
    }
    if matches.is_some_and(|checksum_matched| checksum_matched) {
        log::info!("Downloading the authlib-injector from {url}");
        let download_task = DownloadTask {
            url: url.to_string(),
            file: path,
            size_bytes: None,
            checksum: Checksum::Sha256(sha256.to_string()),
            task_type: DownloadTaskType::AuthlibInjector,
        };
        download::download(&download_task, progress).await?;
        log::info!("The authlib-injector is up to date");
    };
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
