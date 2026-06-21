// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// Configuration for mirror sources used to download libraries and assets.
///
/// You can customize the download URLs for Minecraft libraries and asset files.
/// Each field provides a list of mirrors that will be attempted in order.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorConfig {
    /// Mirror URLs for library files.
    ///
    /// Defaults to:
    /// - `https://libraries.minecraft.net`
    /// - `https://bmclapi2.bangbang93.com/maven`
    pub libraries: Vec<String>,

    /// Mirror URLs for asset files.
    ///
    /// Defaults to:
    /// - `https://resources.download.minecraft.net`
    /// - `https://bmclapi2.bangbang93.com/assets`
    pub assets: Vec<String>,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            libraries: vec![
                "https://libraries.minecraft.net".to_string(),
                "https://bmclapi2.bangbang93.com/maven".to_string(),
            ],
            assets: vec![
                "https://resources.download.minecraft.net".to_string(),
                "https://bmclapi2.bangbang93.com/assets".to_string(),
            ],
        }
    }
}

/// Configuration for controlling download behavior.
///
/// Includes concurrency limits, speed throttling, and mirror settings.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    /// Maximum number of concurrent download tasks.
    ///
    /// This limits how many downloads can happen at the same time (i.e. max connections).
    /// A higher number increases parallelism, but may use more system/network resources.
    /// Default is `100`.
    pub max_connections: usize,

    /// Maximum download speed (in bytes per second).
    ///
    /// A value of `0` disables throttling (unlimited speed).
    pub max_download_speed: u64,

    /// Custom mirror configuration.
    ///
    /// Defines where to download libraries and assets from.
    pub mirror: MirrorConfig,

    pub use_system_proxy: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            max_download_speed: 0,
            mirror: MirrorConfig::default(),
            use_system_proxy: true,
        }
    }
}
