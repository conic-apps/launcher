// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use folder::MinecraftLocation;
use shared::HTTP_CLIENT;
use version::Version;

/// Represents a specific version of a Fabric artifact.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricArtifactVersion {
    /// The Minecraft game version this artifact targets.
    pub game_version: Option<String>,
    /// A separator string used in versioning.
    pub separator: Option<String>,
    /// The build number associated with this artifact version.
    pub build: Option<usize>,
    /// The Maven coordinate string identifying the artifact.
    pub maven: String,
    /// The version string of this artifact.
    pub version: String,
    /// Whether this artifact version is considered stable.
    pub stable: bool,
}

/// Collection of Fabric artifact versions grouped by type.
///
/// Includes mappings and loader artifacts.
#[derive(Deserialize, Serialize)]
pub struct FabricArtifacts {
    /// List of mapping artifact versions.
    pub mappings: Vec<FabricArtifactVersion>,
    /// List of loader artifact versions.
    pub loader: Vec<FabricArtifactVersion>,
}

/// Represents Fabric loader artifacts including loader, intermediary, and launcher metadata.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoaderArtifact {
    /// The Fabric loader artifact version.
    pub loader: FabricArtifactVersion,
    /// The intermediary artifact version.
    pub intermediary: FabricArtifactVersion,
    /// Metadata for the launcher.
    pub launcher_meta: LauncherMeta,
}

/// Wrapper for a list of Yarn artifact versions.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnArtifactList(Vec<FabricArtifactVersion>);

/// Wrapper for a list of Fabric loader artifacts.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderArtifactList(Vec<FabricLoaderArtifact>);

impl LoaderArtifactList {
    /// Asynchronously fetches loader artifacts list for a given Minecraft version.
    ///
    /// # Arguments
    ///
    /// * `mcversion` - The Minecraft version string to query loader artifacts for.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` containing the loaded `LoaderArtifactList` on success.
    pub async fn new(mcversion: &str) -> anyhow::Result<Self> {
        Ok(HTTP_CLIENT
            .get(format!(
                "https://meta.fabricmc.net/v2/versions/loader/{mcversion}"
            ))
            .send()
            .await?
            .json()
            .await?)
    }
}

/// Metadata information for the Fabric launcher.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMeta {
    /// Version number of the launcher metadata.
    pub version: usize,

    pub libraries: LauncherMetaLibraries,

    /// Main class entry point of the launcher, stored as JSON value to accommodate varying formats.
    pub main_class: Value,
}

/// Grouping of launcher libraries categorized by usage context.
#[derive(Deserialize, Serialize)]
pub struct LauncherMetaLibraries {
    pub client: Vec<LauncherMetaLibrariesItems>,
    pub common: Vec<LauncherMetaLibrariesItems>,
    pub server: Vec<LauncherMetaLibrariesItems>,
}

/// Represents an individual launcher library item.
///
/// Each item may have an optional name and URL.
#[derive(Deserialize, Serialize, Clone)]
pub struct LauncherMetaLibrariesItems {
    /// Optional name of the library.
    pub name: Option<String>,
    /// Optional URL to the library resource.
    pub url: Option<String>,
}

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
    let response = HTTP_CLIENT.get(url).send().await.unwrap();
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
