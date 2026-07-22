// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::io::Read;

use download::{Checksum, DownloadTask, DownloadTaskType, task::Progress};
use folder::DATA_LOCATION;
use serde_json::Value;
use sha2::Digest;
use shared::HTTP_CLIENT;

use crate::error::*;

pub async fn ensure_latest(progress: &Progress) -> Result<()> {
    let path = DATA_LOCATION.clone().authlib_injector;
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
    let mut file = std::fs::File::open(&path)?;
    if verify_sha256_from_read(&mut file, sha256).is_some_and(|checksum_matched| checksum_matched) {
        let download_task = DownloadTask {
            url: url.to_string(),
            file: path,
            size_bytes: None,
            checksum: Checksum::Sha256(sha256.to_string()),
            task_type: DownloadTaskType::AuthlibInjector,
        };
        download::download(&download_task, progress).await?;
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
