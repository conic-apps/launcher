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
/// unreadable.
///
/// A missing file is ordinary and silent. A file that exists but cannot be read
/// or parsed is `warn!`ed with its path, because falling back moves every root
/// back to the platform default — which reads as an empty library and accounts
/// the user cannot account for having signed out of.
pub fn load_overrides() -> LocationOverrides {
    let path = bootstrap_path();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return LocationOverrides::default();
        }
        Err(error) => {
            log::warn!(
                "could not read {} ({error}); using the default storage locations",
                path.display()
            );
            return LocationOverrides::default();
        }
    };
    match toml::from_str(&text) {
        Ok(overrides) => overrides,
        Err(error) => {
            log::warn!(
                "{} is not a valid locations file ({error}); using the default storage \
                 locations",
                path.display()
            );
            LocationOverrides::default()
        }
    }
}

/// Writes the bootstrap file, creating its anchor directory.
pub fn save_overrides(overrides: &LocationOverrides) -> io::Result<()> {
    let path = bootstrap_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(overrides).map_err(io::Error::other)?;
    fs::write(&path, text)?;
    // Logged because the write is what a relocation rests on: the app restarts
    // immediately afterwards and reads this back, so a failure here is a
    // relocation that silently did not happen.
    log::info!(
        "wrote {} (launcher: {:?}, minecraft: {:?}, instances: {:?})",
        path.display(),
        overrides.launcher,
        overrides.minecraft,
        overrides.instances
    );
    Ok(())
}
