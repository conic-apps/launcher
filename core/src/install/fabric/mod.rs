// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_plugin_http::reqwest;

pub mod install;
pub use install::install;

#[derive(Debug, Deserialize, Serialize)]
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
///
/// # Example
///
/// basic usage:
///
/// ```rust
/// use aml_core::install::fabric::FabricArtifacts;
///
/// async fn fn_name() {
///     let artifacts = FabricArtifacts::new().await;
///     println!("{:#?}", artifacts);
/// }
/// ```
#[derive(Debug, Deserialize, Serialize)]
pub struct FabricArtifacts {
    pub mappings: Vec<FabricArtifactVersion>,
    pub loader: Vec<FabricArtifactVersion>,
}

/// Fabric Loader Artifact
///
/// # Example
///
/// basic usage:
///
/// ```rust
/// use aml_core::install::fabric::FabricLoaderArtifact;
///
/// async fn fn_name() {
///     let artifact = FabricLoaderArtifact::new("1.19.4", "0.1.0.48").await;
///     println!("{:#?}", artifact);
/// }
/// ```
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoaderArtifact {
    pub loader: FabricArtifactVersion,
    pub intermediary: FabricArtifactVersion,
    pub launcher_meta: LauncherMeta,
}

/// Yarn Artifacts
///
/// # Example
///
/// basic usage:
///
/// ```rust
/// use aml_core::install::fabric::YarnArtifactList;
///
/// async fn fn_name() {
///     let artifacts = YarnArtifactList::new().await;
///     println!("{:#?}", artifacts);
/// }
/// ```
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnArtifactList(Vec<FabricArtifactVersion>);

/// Loader Artifacts
///
/// # Example
///
/// basic usage:
///
/// ```rust
/// use aml_core::install::fabric::LoaderArtifactList;
///
/// async fn fn_name() {
///     let artifacts = LoaderArtifactList::new().await;
///     println!("{:#?}", artifacts);
/// }
/// ```
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderArtifactList(Vec<FabricLoaderArtifact>);

impl LoaderArtifactList {
    /// get loader artifacts
    pub async fn new(mcversion: &str) -> anyhow::Result<Self> {
        Ok(reqwest::get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            mcversion
        ))
        .await?
        .json()
        .await?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMeta {
    pub version: usize,
    pub libraries: LauncherMetaLibraries,
    pub main_class: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LauncherMetaLibraries {
    pub client: Vec<LauncherMetaLibrariesItems>,
    pub common: Vec<LauncherMetaLibrariesItems>,
    pub server: Vec<LauncherMetaLibrariesItems>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LauncherMetaLibrariesItems {
    pub name: Option<String>,
    pub url: Option<String>,
}
