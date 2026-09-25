// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Fabric loader version list (`crates/install/src/fabric.rs`).
//!
//! Only the version list is mirrored; writing the loader profile JSON belongs
//! to the install task, which is not migrated yet.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{HTTP_CLIENT, error::*};

/// Represents a specific version of a Fabric artifact.
#[derive(Deserialize, Serialize, Clone)]
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

/// Represents Fabric loader artifacts including loader, intermediary, and launcher metadata.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoaderArtifact {
    /// The Fabric loader artifact version.
    pub loader: FabricArtifactVersion,
    /// The intermediary artifact version.
    pub intermediary: FabricArtifactVersion,
    /// Metadata for the launcher.
    pub launcher_meta: LauncherMeta,
}

/// Wrapper for a list of Fabric loader artifacts.
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

/// Metadata information for the Fabric launcher.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMeta {
    /// Version number of the launcher metadata.
    pub version: usize,

    pub libraries: LauncherMetaLibraries,

    /// Main class entry point of the launcher, stored as JSON value to accommodate varying formats.
    pub main_class: Value,
}

/// Grouping of launcher libraries categorized by usage context.
#[derive(Deserialize, Serialize, Clone)]
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
