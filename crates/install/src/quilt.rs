// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Quilt loader version list and profile installer.

use serde::{Deserialize, Serialize};

use shared::HTTP_CLIENT;
use storage::MinecraftLocation;
use version::Version;

use crate::error::*;

#[derive(Clone, Deserialize, Serialize)]
pub struct QuiltArtifactVersion {
    // Deserialized but unread; `dead_code` is allowed so they can stay private.
    #[allow(dead_code)]
    separator: String,
    #[allow(dead_code)]
    build: u32,

    /// Maven coordinates, e.g., "org.quiltmc.quilt-loader:0.16.1"
    pub maven: String,
    pub version: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltVersionHashed {
    pub maven: String,
    pub version: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltVersionIntermediary {
    pub maven: String,
    pub version: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QuiltLibraries {
    pub client: Vec<QuiltLibrary>,
    pub common: Vec<QuiltLibrary>,
    pub server: Vec<QuiltLibrary>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltLauncherMeta {
    pub version: u32,
    pub libraries: QuiltLibraries,
    pub main_class: QuiltMainClass,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltMainClass {
    pub client: Option<String>,
    pub server: Option<String>,
    pub server_launcher: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltVersion {
    pub loader: QuiltArtifactVersion,
    pub hashed: Option<QuiltVersionHashed>,
    pub intermediary: Option<QuiltVersionIntermediary>,
    pub launcher_meta: QuiltLauncherMeta,
}

/// Holds a list of available Quilt versions, newest first.
#[derive(Clone, Deserialize, Serialize)]
pub struct QuiltVersionList(Vec<QuiltVersion>);

impl QuiltVersionList {
    /// Fetches the Quilt versions for a Minecraft version, newest first.
    pub async fn new(mcversion: &str) -> Result<Self> {
        let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mcversion}");
        let mut response = HTTP_CLIENT.get(url).send().await?.json::<Self>().await?;
        response
            .0
            .sort_by(|a, b| b.loader.version.cmp(&a.loader.version));
        Ok(response)
    }

    /// The versions, newest first.
    pub fn as_slice(&self) -> &[QuiltVersion] {
        &self.0
    }
}

/// Fetches the Quilt version profile and saves it as the version's
/// `version.json` inside the Minecraft `versions` folder.
pub async fn install(
    mcversion: &str,
    quilt_version: &str,
    minecraft: MinecraftLocation,
) -> Result<()> {
    // Log like Fabric, so the two loaders' install logs match.
    log::info!("Installing quilt {quilt_version} for {mcversion}");
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{mcversion}/{quilt_version}/profile/json"
    );
    let response = HTTP_CLIENT.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        log::warn!("the quilt meta API answered {status} for {quilt_version} on {mcversion}");
    }
    let quilt_version_json: Version = response.json().await?;
    // `version_name` is the id everything else keys off, so it is worth recording.
    let version_name = quilt_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    if let Some(parent) = json_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    log::info!("Saving version metadata file to {}", json_path.display());
    tokio::fs::write(
        json_path,
        serde_json::to_string_pretty(&quilt_version_json)?,
    )
    .await?;
    log::info!("Installed quilt as {version_name}");
    Ok(())
}
