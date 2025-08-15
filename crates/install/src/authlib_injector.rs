// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use download::{Checksum, DownloadTask, DownloadType, task::Progress};
use folder::MinecraftLocation;
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

pub async fn install_latest(
    minecraft_location: &MinecraftLocation,
    version_id: &str,
    progress: &Progress,
) -> Result<()> {
    let path = minecraft_location.get_authlib_injector(version_id);
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
    let download_task = DownloadTask {
        url: url.to_string(),
        file: path,
        checksum: Checksum::Sha256(sha256.to_string()),
        r#type: DownloadType::AuthlibInjector,
    };
    download::download(&download_task, progress).await?;
    Ok(())
}
