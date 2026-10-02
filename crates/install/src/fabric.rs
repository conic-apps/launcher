// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Fabric loader version list and profile installer.

use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use shared::HTTP_CLIENT;
use storage::MinecraftLocation;
use version::Version;

use crate::error::*;

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FabricArtifactVersion {
    pub game_version: Option<String>,
    pub separator: Option<String>,
    pub build: Option<usize>,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Deserialize, Serialize)]
pub struct FabricArtifacts {
    pub mappings: Vec<FabricArtifactVersion>,
    pub loader: Vec<FabricArtifactVersion>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoaderArtifact {
    pub loader: FabricArtifactVersion,
    pub intermediary: FabricArtifactVersion,
    pub launcher_meta: LauncherMeta,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LoaderArtifactList(Vec<FabricLoaderArtifact>);

impl LoaderArtifactList {
    pub async fn new(mcversion: &str) -> Result<Self> {
        Ok(HTTP_CLIENT
            .get(format!(
                "https://meta.fabricmc.net/v2/versions/loader/{mcversion}"
            ))
            .send()
            .await?
            .json()
            .await?)
    }

    /// The loader artifacts, in the order the Fabric meta API returned them.
    pub fn as_slice(&self) -> &[FabricLoaderArtifact] {
        &self.0
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnArtifactList(Vec<FabricArtifactVersion>);

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMeta {
    pub version: usize,

    pub libraries: LauncherMetaLibraries,

    /// Main class entry point; a JSON value because Fabric may emit it in
    /// varying formats.
    pub main_class: Value,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LauncherMetaLibraries {
    pub client: Vec<LauncherMetaLibrariesItems>,
    pub common: Vec<LauncherMetaLibrariesItems>,
    pub server: Vec<LauncherMetaLibrariesItems>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LauncherMetaLibrariesItems {
    pub name: Option<String>,
    pub url: Option<String>,
}

/// Fetches the Fabric version profile and saves it as the version's
/// `version.json` inside the Minecraft `versions` folder.
pub async fn install(
    mcversion: &str,
    fabric_version: &str,
    minecraft: MinecraftLocation,
) -> Result<()> {
    info!("Saving version metadata file");
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{mcversion}/{fabric_version}/profile/json"
    );
    let response = HTTP_CLIENT.get(url).send().await?;
    let fabric_version_json: Version = response.json().await?;
    let version_name = fabric_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    if let Some(parent) = json_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(
        json_path,
        serde_json::to_string_pretty(&fabric_version_json)?,
    )
    .await?;
    Ok(())
}
