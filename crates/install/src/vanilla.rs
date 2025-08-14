// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::str::FromStr;

use serde_json::Value;
use shared::HTTP_CLIENT;

use download::{DownloadTask, DownloadType};
use folder::MinecraftLocation;
use version::{self, AssetIndexObject, ResolvedLibrary, ResolvedVersion, resolve_version};

use serde::{Deserialize, Serialize};

use crate::error::*;

#[derive(Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: LatestVersion,
    pub versions: Vec<VersionInfo>,
}

impl VersionManifest {
    pub async fn new() -> Result<VersionManifest> {
        // Not allow custom source to avoid attack
        Ok(HTTP_CLIENT
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await?
            .json()
            .await?)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LatestVersion {
    pub release: String,
    pub snapshot: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: u8,
}

/// Generate a complete list of required files to download for the specified Minecraft version,
/// including version metadata, libraries, assets, and client JAR.
///
/// # Arguments
///
/// * `version_id` - The Minecraft version ID (e.g., `"1.20.1"`).
/// * `minecraft_location` - The root location of the Minecraft installation.
///
/// # Returns
///
/// A vector of [`Download`] entries describing what files need to be downloaded.
pub async fn generate_download_info(
    version_id: &str,
    minecraft_location: MinecraftLocation,
) -> Result<Vec<DownloadTask>> {
    let raw_version_json = get_version_json(version_id).await?;
    let resolved_version = resolve_version(
        &version::Version::from_str(&raw_version_json)?,
        &minecraft_location,
        &[],
    )
    .await?;
    let resolved_version_id = &resolved_version.id;

    save_version_json(&minecraft_location, resolved_version_id, &raw_version_json).await?;

    let client_download_task =
        generate_client_download_task(&minecraft_location, &resolved_version)?;
    let libraries_download_task =
        generate_libraries_downloads(&minecraft_location, &resolved_version);
    let assets_download_task =
        generate_assets_downloads(&minecraft_location, &resolved_version).await?;

    let mut download_info = vec![];
    download_info.push(client_download_task);
    download_info.extend(libraries_download_task);
    download_info.extend(assets_download_task);

    override_log4j2_configuration_file(&minecraft_location, &resolved_version).await?;

    Ok(download_info)
}

async fn get_version_json(version_id: &str) -> Result<String> {
    let versions = VersionManifest::new().await?.versions;
    let filtered_version_metadata = versions
        .into_iter()
        .filter(|v| v.id == version_id)
        .collect::<Vec<_>>();
    let version_metadata = filtered_version_metadata
        .first()
        .ok_or(Error::VersionMetadataNotfound)?;
    Ok(HTTP_CLIENT
        .get(version_metadata.url.clone())
        .send()
        .await?
        .text()
        .await?)
}

async fn save_version_json(
    minecraft_location: &MinecraftLocation,
    resolved_version_id: &str,
    raw_version_json: &str,
) -> Result<()> {
    let version_json_path = minecraft_location
        .versions
        .join(format!("{resolved_version_id}/{resolved_version_id}.json"));
    if let Some(parent) = version_json_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    async_fs::write(&version_json_path, raw_version_json.as_bytes()).await?;
    Ok(())
}

fn generate_client_download_task(
    minecraft_location: &MinecraftLocation,
    resolved_version: &ResolvedVersion,
) -> Result<DownloadTask> {
    let downloads = resolved_version.downloads.clone();
    let client = downloads
        .get("client")
        .ok_or(Error::InvalidVersionJson("client".to_string()))?;
    let id = &resolved_version.id;
    Ok(DownloadTask {
        url: format!(
            "https://piston-data.mojang.com/v1/objects/{}/client.jar",
            client.sha1
        ),
        file: minecraft_location.versions.join(format!("{id}/{id}.jar")),
        sha1: Some(client.sha1.to_string()),
        r#type: DownloadType::Unknown,
    })
}

/// Generate download entries for all resolved libraries.
///
/// # Arguments
///
/// * `libraries` - A slice of resolved libraries to download.
/// * `minecraft_location` - The Minecraft installation directory.
///
/// # Returns
///
/// A vector of [`Download`] objects describing library files to download.
pub fn generate_libraries_downloads(
    minecraft_location: &MinecraftLocation,
    resolved_version: &ResolvedVersion,
) -> Vec<DownloadTask> {
    let libraries = resolved_version.libraries.clone();
    libraries
        .into_iter()
        .map(|library| {
            let library_download_info = match library {
                ResolvedLibrary::Native(download_info) => download_info,
                ResolvedLibrary::Common(download_info) => download_info,
            };
            DownloadTask {
                url: library_download_info.url,
                file: minecraft_location
                    .libraries
                    .join(library_download_info.path),
                sha1: library_download_info.sha1,
                r#type: DownloadType::Libraries,
            }
        })
        .collect()
}

/// Generate download entries for all asset files from the asset index.
///
/// # Arguments
///
/// * `asset_index` - The asset index containing metadata of assets.
/// * `minecraft_location` - The Minecraft installation directory.
///
/// # Returns
///
/// A vector of [`Download`] objects for assets, including the index file itself.
pub async fn generate_assets_downloads(
    minecraft_location: &MinecraftLocation,
    resolved_version: &ResolvedVersion,
) -> Result<Vec<DownloadTask>> {
    let asset_index = resolved_version
        .asset_index
        .clone()
        .ok_or(Error::InvalidVersionJson("assetIndex".to_string()))?;
    let asset_index_raw = HTTP_CLIENT
        .get(&asset_index.url)
        .send()
        .await?
        .text()
        .await?;
    let asset_index_json: Value = serde_json::from_str(asset_index_raw.as_ref())?;
    let asset_index_object: AssetIndexObject =
        serde_json::from_value(asset_index_json["objects"].clone())?;
    let mut assets: Vec<_> = asset_index_object
        .into_iter()
        .map(|obj| DownloadTask {
            url: format!(
                "https://resources.download.minecraft.net/{}/{}",
                &obj.1.hash[0..2],
                obj.1.hash
            ),
            file: minecraft_location
                .assets
                .join("objects")
                .join(&obj.1.hash[0..2])
                .join(&obj.1.hash),
            sha1: Some(obj.1.hash),
            r#type: DownloadType::Unknown,
        })
        .collect();
    assets.push(DownloadTask {
        url: asset_index.url,
        file: minecraft_location.get_assets_index(&asset_index.id),
        sha1: None,
        r#type: DownloadType::Unknown,
    });
    Ok(assets)
}

const LOF4J2_CONFIGURATION: &[u8] = include_bytes!("./log4j2.xml");

/// Override the `log4j2.xml` configuration file for the given version.
///
/// # Arguments
///
/// * `version` - The resolved Minecraft version.
/// * `minecraft_location` - The Minecraft installation directory.
///
/// # Returns
///
/// An empty [`Result`] indicating success or failure.
pub async fn override_log4j2_configuration_file(
    minecraft_location: &MinecraftLocation,
    version: &ResolvedVersion,
) -> Result<()> {
    async_fs::write(
        minecraft_location.get_log_config(version.id.clone()),
        LOF4J2_CONFIGURATION,
    )
    .await?;
    Ok(())
}
