// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// Configuration related to background music.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicConfig {
    /// Whether background music is enabled.
    pub enabled: bool,

    /// Whether to resume the last playing track and its position on startup.
    pub resume_on_startup: bool,

    /// Whether to show the audio visualizer in the game view footer.
    pub show_visualizer: bool,

    pub main_volumn: u8,
    pub main_volumn_background: u8,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            resume_on_startup: true,
            show_visualizer: true,
            main_volumn: 100,
            main_volumn_background: 25,
        }
    }
}
