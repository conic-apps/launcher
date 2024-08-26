// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use tauri_plugin_http::reqwest;

use crate::{folder::MinecraftLocation, version::Version, HTTP_CLIENT};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuiltArtifactVersion {
    separator: String,
    build: u32,

    /// e.g. "org.quiltmc.quilt-loader:0.16.1"
    maven: String,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltVersionHashed {
    pub maven: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltVersionIntermediary {
    pub maven: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltLibraries {
    pub client: Vec<QuiltLibrary>,
    pub common: Vec<QuiltLibrary>,
    pub server: Vec<QuiltLibrary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltLauncherMeta {
    pub version: u32,
    pub libraries: QuiltLibraries,
    pub main_class: QuiltMainClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltMainClass {
    pub client: Option<String>,
    pub server: Option<String>,
    pub server_launcher: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltVersion {
    pub loader: QuiltArtifactVersion,
    pub hashed: QuiltVersionHashed,
    pub intermediary: QuiltVersionIntermediary,
    pub launcher_meta: QuiltLauncherMeta,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuiltVersionList(Vec<QuiltVersion>);
impl QuiltVersionList {
    pub async fn new(mcversion: &str) -> anyhow::Result<Self> {
        let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mcversion}");
        let response = HTTP_CLIENT.get().unwrap().get(url).send().await?;
        Ok(response.json().await?)
    }
}

/// Save the quilt `version.json`
pub async fn install(
    mcversion: &str,
    quilt_version: &str,
    minecraft: MinecraftLocation,
) -> anyhow::Result<()> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{mcversion}/{quilt_version}/profile/json"
    );
    let response = reqwest::get(url).await.unwrap();
    let quilt_version_json: Version = response.json().await.unwrap();
    let version_name = quilt_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    tokio::fs::create_dir_all(json_path.parent().unwrap()).await?;
    tokio::fs::write(
        json_path,
        serde_json::to_string_pretty(&quilt_version_json).unwrap(),
    )
    .await?;
    Ok(())
}
