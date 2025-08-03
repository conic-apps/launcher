// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_plugin_http::reqwest;

/// Represents a single Forge version entry returned from the version list API.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionListItem {
    pub _id: String,
    pub build: u32,
    pub __v: u32,
    pub version: String,
    pub modified: String,
    pub mcversion: String,
    pub files: Vec<ForgeInstallerFile>,
    pub branch: Option<Value>,
}

/// Represents a single installer file entry in a Forge version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeInstallerFile {
    pub format: String,
    pub category: String,
    pub hash: Option<String>,
}

/// A list of Forge versions for a given Minecraft version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionList(Vec<ForgeVersionListItem>);

impl ForgeVersionList {
    /// Fetches the Forge version list for a specified Minecraft version.
    ///
    /// # Arguments
    ///
    /// * `mcversion` - The target Minecraft version (e.g., "1.20.1").
    ///
    /// # Returns
    ///
    /// A `ForgeVersionList` containing all available Forge versions for the specified Minecraft version.
    pub async fn new(mcversion: &str) -> Result<Self> {
        Ok(reqwest::get(format!(
            "https://bmclapi2.bangbang93.com/forge/minecraft/{mcversion}"
        ))
        .await?
        .json::<Self>()
        .await?)
    }
}
