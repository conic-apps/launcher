// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::str::FromStr;

use anyhow::{anyhow, Result};
use serde_json::Value;
use tauri_plugin_http::reqwest;
use tokio::io::AsyncWriteExt;

use crate::download::{Download, DownloadType};
use crate::version::ResolvedLibrary;
use crate::HTTP_CLIENT;
use crate::{
    folder::MinecraftLocation,
    version::{self, AssetIndex, AssetIndexObject, ResolvedVersion, VersionManifest},
};

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
pub(crate) fn generate_libraries_downloads(
    libraries: &[ResolvedLibrary],
    minecraft_location: &MinecraftLocation,
) -> Vec<Download> {
    libraries
        .iter()
        .cloned()
        .map(|library| Download {
            url: library.download_info.url,
            file: minecraft_location
                .libraries
                .join(library.download_info.path),
            sha1: library.download_info.sha1,
            r#type: DownloadType::Unknown,
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
    asset_index: AssetIndex,
    minecraft_location: &MinecraftLocation,
) -> Result<Vec<Download>> {
    let asset_index_url = reqwest::Url::parse(asset_index.url.as_ref())?;
    let asset_index_raw = reqwest::get(asset_index_url).await?.text().await?;
    let asset_index_json: Value = serde_json::from_str(asset_index_raw.as_ref())?;
    let asset_index_object: AssetIndexObject =
        serde_json::from_value(asset_index_json["objects"].clone())?;
    let mut assets: Vec<_> = asset_index_object
        .into_iter()
        .map(|obj| Download {
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
    assets.push(Download {
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
    version: &ResolvedVersion,
    minecraft_location: &MinecraftLocation,
) -> Result<()> {
    tokio::fs::write(
        minecraft_location.get_log_config(version.id.clone()),
        LOF4J2_CONFIGURATION,
    )
    .await?;
    Ok(())
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
) -> Result<Vec<Download>> {
    let versions = VersionManifest::new().await?.versions;
    let version_metadata: Vec<_> = versions
        .into_iter()
        .filter(|v| v.id == version_id)
        .collect();
    if version_metadata.len() != 1 {
        return Err(anyhow!("Bad version manifest"));
    };
    let version_metadata = version_metadata.first().unwrap();
    let version_json_raw = HTTP_CLIENT
        .get(version_metadata.url.clone())
        .send()
        .await?
        .text()
        .await?;
    let version = version::Version::from_str(&version_json_raw)?
        .resolve(&minecraft_location, &[])
        .await?;
    let id = &version.id;

    let version_json_path = minecraft_location.versions.join(format!("{id}/{id}.json"));
    tokio::fs::create_dir_all(version_json_path.parent().unwrap()).await?;
    let mut file = tokio::fs::File::create(&version_json_path).await?;
    file.write_all(version_json_raw.as_bytes()).await?;

    let mut download_list = vec![];
    let downloads = version.downloads.clone();
    let client = downloads.get("client").ok_or(anyhow!("No client found!"))?;
    download_list.push(Download {
        url: format!(
            "https://piston-data.mojang.com/v1/objects/{}/client.jar",
            client.sha1
        ),
        file: minecraft_location.versions.join(format!("{id}/{id}.jar")),
        sha1: Some(client.sha1.to_string()),
        r#type: DownloadType::Unknown,
    });
    download_list.extend(generate_libraries_downloads(
        &version.libraries,
        &minecraft_location,
    ));
    download_list.extend(
        generate_assets_downloads(
            version
                .asset_index
                .clone()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?,
            &minecraft_location,
        )
        .await?,
    );
    let _ = override_log4j2_configuration_file(&version, &minecraft_location).await;
    Ok(download_list)
}
