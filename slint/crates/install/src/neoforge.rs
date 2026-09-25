// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Neoforge version list (`crates/install/src/neoforge.rs`).
//!
//! Only the version list is mirrored; downloading and running the installer
//! belongs to the install task, which is not migrated yet.

use serde_json::Value;

use crate::{HTTP_CLIENT, error::*};

/// Fetches every published Neoforge version, newest first.
///
/// The modem versions live under `net/neoforged/neoforge`, the pre-1.20.2
/// ("legacy") ones under `net/neoforged/forge`; both are appended into one
/// list, like the original.
pub async fn get_neoforge_version_list() -> Result<Vec<String>> {
    let legacy_versions = HTTP_CLIENT
        .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge")
        .send()
        .await?
        .json::<Value>()
        .await?["versions"]
        .clone();
    let legacy_versions = serde_json::from_value::<Vec<String>>(legacy_versions)?;
    let modern_versions = HTTP_CLIENT
        .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge")
        .send()
        .await?
        .json::<Value>()
        .await?["versions"]
        .clone();
    let mut modern_versions = serde_json::from_value::<Vec<String>>(modern_versions)?;
    modern_versions.extend(legacy_versions);
    Ok(modern_versions)
}
