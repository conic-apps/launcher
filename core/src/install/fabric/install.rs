// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use crate::{folder::MinecraftLocation, version::Version};
use log::info;
use tauri_plugin_http::reqwest;

/// Downloads and saves the Fabric version metadata JSON file.
///
/// This function fetches the version metadata from the Fabric Meta API based on the specified
/// Minecraft version and Fabric loader version. It saves the resulting `version.json`
/// to the appropriate location inside the `.minecraft/versions` folder.
///
/// # Arguments
///
/// * `mcversion` - The target Minecraft version (e.g., `"1.20.1"`).
/// * `fabric_version` - The loader version to be used (e.g., `"0.14.21"`).
/// * `minecraft` - The local Minecraft installation location.
///
/// # Returns
///
/// Returns `Ok(())` if the file is successfully written, or an `anyhow::Error` if any step fails.
///
/// # Remarks
///
/// After calling this function, you should revalidate the libraries used by the version
/// before launching the game, to ensure integrity and compatibility.
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
