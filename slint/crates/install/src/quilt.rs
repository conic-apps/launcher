// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Quilt loader version list (`crates/install/src/quilt.rs`).
//!
//! Only the version list is mirrored; writing the loader profile JSON belongs
//! to the install task, which is not migrated yet.

use serde::{Deserialize, Serialize};

use crate::{HTTP_CLIENT, error::*};

/// Represents a Quilt loader artifact version, including its Maven coordinates and version.
#[derive(Clone, Deserialize, Serialize)]
pub struct QuiltArtifactVersion {
    // Kept private like the original: only `version` is read by the frontend.
    #[allow(dead_code)]
    separator: String,
    #[allow(dead_code)]
    build: u32,

    /// Maven coordinates, e.g., "org.quiltmc.quilt-loader:0.16.1"
    pub maven: String,
    pub version: String,
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
    pub hashed: Option<QuiltVersionHashed>,
    pub intermediary: Option<QuiltVersionIntermediary>,
    pub launcher_meta: QuiltLauncherMeta,
}

/// Holds a list of available Quilt versions, newest first.
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
    pub async fn new(mcversion: &str) -> Result<Self> {
        let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mcversion}");
        let mut response = HTTP_CLIENT.get(url).send().await?.json::<Self>().await?;
        response
            .0
            .sort_by(|a, b| b.loader.version.cmp(&a.loader.version));
        Ok(response)
    }

    /// The versions, in the order the Quilt meta API returned them (newest first).
    pub fn as_slice(&self) -> &[QuiltVersion] {
        &self.0
    }
}
