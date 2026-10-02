// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher's background music player.
//!
//! Everything the player needs — decoding, the output device, the analyser, the
//! transport, the playlist, the saved position — lives here, split by
//! responsibility:
//!
//!   * [`decode`] turns a file into PCM.
//!   * [`analyser`] is the frequency analyser the footer visualizer reads, down
//!     to the Blackman window and the smoothing constant.
//!   * [`player`] is the audio graph (`source → gain → destination`, with the
//!     analyser tapping the post-gain output) and the transport driving it.
//!   * [`session`] persists the current track and position.
//!
//! [`list_music_files`] reads the shared music folder
//! (`storage::LOCATIONS.launcher.music`).

use std::path::Path;

use log::warn;

use error::Result;

pub mod analyser;
pub mod decode;
pub mod error;
pub mod player;
pub mod session;

pub use error::Error;
pub use player::{PERSIST_INTERVAL, Player, PlayerState};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "ogg", "flac", "m4a", "aac", "opus", "wma", "aiff", "aif",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicFile {
    pub name: String,
    pub path: String,
}

/// Lists the files in the music directory that carry a recognised audio
/// extension.
pub fn list_music_files() -> Result<Vec<MusicFile>> {
    let entries = std::fs::read_dir(&storage::LOCATIONS.launcher.music)?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warn!("Could not read music directory entry: {error}");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() || !is_supported_extension(&path) {
            continue;
        }
        let name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        files.push(MusicFile {
            name,
            path: path.to_string_lossy().to_string(),
        });
    }
    files.sort_by_key(|a| a.name.to_lowercase());
    Ok(files)
}

fn is_supported_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    let extension = extension.to_lowercase();
    SUPPORTED_EXTENSIONS
        .iter()
        .any(|&supported| supported == extension)
}
