// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The saved track and playback position (src/store/music.ts's `localStorage`).
//!
//! The Vue kept `{ path, currentTime }` under `conic.music.lastTrack` in the
//! webview's local storage, which is per-webview and therefore invisible to
//! anything else. A native app has no such store, so the same two fields go to
//! a file next to `config.toml` in the shared data directory. It is the only
//! part of the music player the two frontends do not share: the Tauri app
//! cannot read this file, and this one cannot read the webview's storage.

use std::path::PathBuf;

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};

/// The file the state is kept in, under the data directory's root.
fn session_file() -> PathBuf {
    DATA_LOCATION.root.join("music_session.json")
}

/// The `{ path, currentTime }` pair `SAVED_TRACK_KEY` held.
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
/// The Vue wrapped the write in a `try`/`catch` and ignored the failure
/// (`localStorage may be unavailable`); the same is done here, with the reason
/// logged instead of swallowed.
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
/// Every failure — a missing file, a truncated write, a `currentTime` that is
/// not a number — is "no saved state", as in the Vue's `loadTrackState`.
pub fn load() -> Option<SavedTrack> {
    let raw = std::fs::read(session_file()).ok()?;
    let state: SavedTrack = serde_json::from_slice(&raw).ok()?;
    if state.path.is_empty() || !state.current_time.is_finite() {
        return None;
    }
    Some(state)
}
