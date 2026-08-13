// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Music file listing for the launcher background music player.

use std::path::Path;

use folder::DATA_LOCATION;
use log::warn;
use serde::Serialize;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};

use error::Result;

pub mod error;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "ogg", "flac", "m4a", "aac", "opus", "wma", "aiff", "aif",
];

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("music")
        .invoke_handler(tauri::generate_handler![cmd_list_music_files])
        .build()
}

#[derive(Debug, Clone, Serialize)]
pub struct MusicFile {
    pub name: String,
    pub path: String,
}

#[command]
fn cmd_list_music_files() -> Result<Vec<MusicFile>> {
    list_music_files()
}

/// Lists all supported audio files inside the music directory.
pub fn list_music_files() -> Result<Vec<MusicFile>> {
    let entries = std::fs::read_dir(&DATA_LOCATION.music)?;
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
