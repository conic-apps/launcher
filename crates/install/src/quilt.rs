// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;

use folder::MinecraftLocation;
use version::Version;

/// Represents a Quilt loader artifact version, including its Maven coordinates and version.
#[derive(Clone, Deserialize, Serialize)]
pub struct QuiltArtifactVersion {
    separator: String,
    build: u32,

    /// Maven coordinates, e.g., "org.quiltmc.quilt-loader:0.16.1"
    maven: String,
    version: String,
}

/// Represents a hashed Quilt version, with Maven coordinates.
#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltVersionHashed {
    pub maven: String,
    pub version: String,
}

/// Represents the intermediary mapping version for Quilt.
#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltVersionIntermediary {
    pub maven: String,
    pub version: String,
}

/// Represents a single Quilt library with its name and URL.
#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltLibrary {
    pub name: String,
    pub url: String,
}

/// Represents the categorized libraries required by the Quilt launcher.
#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltLibraries {
    pub client: Vec<QuiltLibrary>,
    pub common: Vec<QuiltLibrary>,
    pub server: Vec<QuiltLibrary>,
}

/// Contains metadata required to launch Quilt, including main classes and libraries.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltLauncherMeta {
    pub version: u32,
    pub libraries: QuiltLibraries,
    pub main_class: QuiltMainClass,
}

/// Holds main class information used to launch different environments.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltMainClass {
    pub client: Option<String>,
    pub server: Option<String>,
    pub server_launcher: Option<String>,
}

/// Represents a complete Quilt version, including loader, intermediary, hashed versions, and metadata.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltVersion {
    pub loader: QuiltArtifactVersion,
    pub hashed: QuiltVersionHashed,
    pub intermediary: QuiltVersionIntermediary,
    pub launcher_meta: QuiltLauncherMeta,
}

/// Holds a list of available Quilt versions.
#[derive(Clone, Deserialize, Serialize)]
pub struct QuiltVersionList(Vec<QuiltVersion>);

impl QuiltVersionList {
    /// Fetches the list of Quilt versions for a specific Minecraft version.
    ///
    /// # Arguments
    ///
    /// * `mcversion` - The target Minecraft version to fetch Quilt versions for.
    ///
    /// # Returns
    ///
    /// * A `QuiltVersionList` containing all available Quilt versions for the given Minecraft version.
    pub async fn new(mcversion: &str) -> anyhow::Result<Self> {
        let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mcversion}");
        let response = HTTP_CLIENT.get(url).send().await?;
        Ok(response.json().await?)
    }
}

/// Downloads and installs the Quilt version metadata into the Minecraft directory.
///
/// This will save the version profile JSON in the appropriate location inside the Minecraft folder.
///
/// # Arguments
///
/// * `mcversion` - Target Minecraft version.
/// * `quilt_version` - Specific Quilt loader version to install.
/// * `minecraft` - Path to the user's Minecraft installation.
///
/// # Returns
///
/// * A `Result<()>` indicating success or failure.
pub async fn install(
    mcversion: &str,
    quilt_version: &str,
    minecraft: MinecraftLocation,
) -> anyhow::Result<()> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{mcversion}/{quilt_version}/profile/json"
    );
    let response = HTTP_CLIENT.get(url).send().await.unwrap();
    let quilt_version_json: Version = response.json().await.unwrap();
    let version_name = quilt_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    async_fs::create_dir_all(json_path.parent().unwrap()).await?;
    async_fs::write(
        json_path,
        serde_json::to_string_pretty(&quilt_version_json).unwrap(),
    )
    .await?;
    Ok(())
}
