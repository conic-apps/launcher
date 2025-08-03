// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    #[serde(default = "default_libraries")]
    pub libraries: Vec<String>,
    #[serde(default = "default_assets")]
    pub assets: Vec<String>,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            libraries: default_libraries(),
            assets: default_assets(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default)]
    pub max_download_speed: usize,
    #[serde(default)]
    /// User custom mirrors
    pub mirror: MirrorConfig,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            max_download_speed: 0,
            mirror: MirrorConfig::default(),
        }
    }
}

fn default_max_connections() -> usize {
    100
}

fn default_assets() -> Vec<String> {
    vec![
        "https://resources.download.minecraft.net".to_string(),
        "https://bmclapi2.bangbang93.com/assets".to_string(),
    ]
}

fn default_libraries() -> Vec<String> {
    vec![
        "https://libraries.minecraft.net".to_string(),
        "https://bmclapi2.bangbang93.com/maven".to_string(),
    ]
}
