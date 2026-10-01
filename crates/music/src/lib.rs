// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/music`, the launcher background music player.
//!
//! The original crate is a Tauri plugin with a single command, the listing of
//! the music folder. Everything else the player did — decoding, the output
//! device, the analyser, the transport, the playlist, the saved position —
//! lived in the webview (`src/store/music.ts` and the `<audio>` element it drove
//! through the Web Audio API). A native app has no webview, so all of it is here,
//! split the way the store's responsibilities were:
//!
//!   * [`decode`] is the `src` the element was pointed at: a file becomes PCM.
//!   * [`analyser`] is the `AnalyserNode`, down to the Blackman window and the
//!     smoothing constant.
//!   * [`player`] is the graph (`source → analyser → gain → destination`) and the
//!     `useMusicStore` actions driving it.
//!   * [`session`] is the `localStorage` entry the store kept its position in.
//!
//! The listing itself is the original's, unchanged, and the folder it reads is
//! the shared one (`slint_folder::DATA_LOCATION.music`), so both frontends see
//! the same files.

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

/// Lists all supported audio files inside the music directory.
pub fn list_music_files() -> Result<Vec<MusicFile>> {
    let entries = std::fs::read_dir(&slint_folder::DATA_LOCATION.music)?;
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
