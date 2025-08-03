// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use crate::{folder::MinecraftLocation, version::Version};
use log::info;
use tauri_plugin_http::reqwest;

/// Save fabric version.json
///
/// Note: You'll need to recheck the library integrity before launching the game.
pub async fn install(
    mcversion: &str,
    fabric_version: &str,
    minecraft: MinecraftLocation,
) -> anyhow::Result<()> {
    info!("Saving version metadata file");
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{mcversion}/{fabric_version}/profile/json"
    );
    let response = reqwest::get(url).await.unwrap();
    let fabric_version_json: Version = response.json().await.unwrap();
    let version_name = fabric_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    tokio::fs::create_dir_all(json_path.parent().unwrap()).await?;
    tokio::fs::write(
        json_path,
        serde_json::to_string_pretty(&fabric_version_json).unwrap(),
    )
    .await?;
    Ok(())
}
