// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// Configuration for mirror sources used to download libraries and assets.
///
/// The download URLs for Minecraft libraries and asset files can be
/// customized. Each field provides a list of mirror URLs; the downloader picks
/// the one with the fewest active connections and falls back to another if a
/// download through it fails.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MirrorConfig {
    /// Mirror URLs for library files.
    pub libraries: Vec<String>,

    /// Mirror URLs for asset files.
    pub assets: Vec<String>,
}

/// Configuration for controlling download behavior.
///
/// Includes a connection cap, speed throttling, and mirror settings.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    /// Maximum number of concurrent download tasks.
    ///
    /// Stored as a launcher setting and surfaced in the UI, but the concurrent
    /// downloader runs a fixed eight tasks at a time, so this value is not
    /// consulted. Default is `100`.
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
