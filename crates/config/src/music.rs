// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicConfig {
    pub enabled: bool,

    pub resume_on_startup: bool,

    pub show_visualizer: bool,

    pub pause_on_launch: bool,

    pub main_volumn: u8,
    pub main_volumn_background: u8,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            resume_on_startup: true,
            show_visualizer: true,
            pause_on_launch: false,
            main_volumn: 100,
            main_volumn_background: 25,
        }
    }
}
