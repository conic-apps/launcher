// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct DownloadConfig {
    #[serde(default = "default_max_connection")]
    pub max_connection: usize,
    #[serde(default)]
    pub max_download_speed: usize,
}

fn default_max_connection() -> usize {
    100
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_connection: default_max_connection(),
            max_download_speed: 0,
        }
    }
}
