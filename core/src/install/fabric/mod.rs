// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_plugin_http::reqwest;

pub mod install;
pub use install::install;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricArtifactVersion {
    pub game_version: Option<String>,
    pub separator: Option<String>,
    pub build: Option<usize>,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

/// Fabric Artifacts
#[derive(Deserialize, Serialize)]
pub struct FabricArtifacts {
    pub mappings: Vec<FabricArtifactVersion>,
    pub loader: Vec<FabricArtifactVersion>,
}

/// Fabric Loader Artifact
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoaderArtifact {
    pub loader: FabricArtifactVersion,
    pub intermediary: FabricArtifactVersion,
    pub launcher_meta: LauncherMeta,
}

/// Yarn Artifacts
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnArtifactList(Vec<FabricArtifactVersion>);

/// Loader Artifacts
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderArtifactList(Vec<FabricLoaderArtifact>);

impl LoaderArtifactList {
    /// get loader artifacts
    pub async fn new(mcversion: &str) -> anyhow::Result<Self> {
        Ok(reqwest::get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{mcversion}"
        ))
        .await?
        .json()
        .await?)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMeta {
    pub version: usize,
    pub libraries: LauncherMetaLibraries,
    pub main_class: Value,
}

#[derive(Deserialize, Serialize)]
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
