// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Mojang version manifest (`crates/install/src/vanilla.rs`).
//!
//! Only the manifest lookup is mirrored; resolving a version into download
//! tasks belongs to the install task, which is not migrated yet.

use serde::{Deserialize, Serialize};

use crate::{HTTP_CLIENT, error::*};

#[derive(Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: LatestVersion,
    pub versions: Vec<VersionInfo>,
}

impl VersionManifest {
    pub async fn new() -> Result<VersionManifest> {
        // Not allow custom source to avoid attack
        Ok(HTTP_CLIENT
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await?
            .json()
            .await?)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LatestVersion {
    pub release: String,
    pub snapshot: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: u8,
}
