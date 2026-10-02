// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The bootstrap file that records the user's location choices.
//!
//! It lives at [`bootstrap_path`] — a fixed platform anchor — rather than in the
//! launcher location, because the launcher location is what it chooses: reading
//! it has to work before the config directory is known.
//!
//! A location left out keeps its default (the `root`-derived one), so the file
//! stays empty until the user changes something.

use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::platform::platform_anchor;

const FILE_NAME: &str = "locations.toml";

/// The overrides a user chose, one per location, absent when default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationOverrides {
    /// Whether the wizard's storage step has been answered — by finishing or
    /// skipping the wizard, or by a change in Settings. The wizard reads it to
    /// decide whether to mount that step at all, so it is not asked twice. It
    /// defaults to `false` for a file written before the key existed.
    #[serde(default)]
    pub initialized: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launcher: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instances: Option<PathBuf>,
}

/// The fixed path of the bootstrap file.
pub fn bootstrap_path() -> PathBuf {
    platform_anchor().join(FILE_NAME)
}

/// Reads the bootstrap file, falling back to no overrides when it is missing or
/// unreadable. A broken file must not keep the launcher from starting.
pub fn load_overrides() -> LocationOverrides {
    match fs::read_to_string(bootstrap_path()) {
        Ok(text) => toml::from_str(&text).unwrap_or_default(),
        Err(_) => LocationOverrides::default(),
    }
}

/// Writes the bootstrap file, creating its anchor directory.
pub fn save_overrides(overrides: &LocationOverrides) -> io::Result<()> {
    let path = bootstrap_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(overrides).map_err(io::Error::other)?;
    fs::write(path, text)
}
