// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct DownloadConfig {
    pub max_connection: usize,
    pub max_download_speed: usize,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_connection: 100,
            max_download_speed: 0,
        }
    }
}
