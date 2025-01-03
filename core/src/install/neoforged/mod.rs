// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

use crate::HTTP_CLIENT;

#[derive(Deserialize, Serialize, Clone)]
pub struct NeoforgedVersionList {
    #[serde(rename = "isSnapshot")]
    pub is_snapshot: bool,
    pub versions: Vec<String>,
}

impl NeoforgedVersionList {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(HTTP_CLIENT
            .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge")
            .send()
            .await?
            .json()
            .await?)
    }
}
