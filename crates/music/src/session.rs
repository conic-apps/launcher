// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The saved track and playback position.
//!
//! The two fields `{ path, current_time }` go to a JSON file next to
//! `config.toml` in the shared data directory.

use std::path::PathBuf;

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};

/// The file the state is kept in, under the data directory's root.
fn session_file() -> PathBuf {
    DATA_LOCATION.root.join("music_session.json")
}

/// The saved track and its position.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SavedTrack {
    /// Absolute path of the track that was playing.
    pub path: String,
    /// How far into it, in seconds.
    #[serde(default)]
    pub current_time: f64,
}

/// Writes the state.
///
/// A failed write is logged rather than propagated: losing the saved position
/// must not interrupt playback.
pub fn save(state: &SavedTrack) {
    let path = session_file();
    if let Err(error) = std::fs::write(&path, serde_json::to_vec(state).unwrap_or_default()) {
        log::warn!(
            "could not save the music session to {}: {error}",
            path.display()
        );
    }
}

/// Reads the state back, or `None` when there is none to read.
///
/// Every failure — a missing file, a truncated write, a `current_time` that is
/// not a number — means "no saved state" rather than an error.
pub fn load() -> Option<SavedTrack> {
    let raw = std::fs::read(session_file()).ok()?;
    let state: SavedTrack = serde_json::from_slice(&raw).ok()?;
    if state.path.is_empty() || !state.current_time.is_finite() {
        return None;
    }
    Some(state)
}
