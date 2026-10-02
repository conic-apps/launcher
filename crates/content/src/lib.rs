// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance's local content.
//!
//! Each module is a self-contained reader of one content kind:
//!
//!   * `mods/` reads the mods folder and resolves each jar's metadata, offline
//!     from the archive itself and online through Modrinth and CurseForge;
//!   * `saves/` reads `level.dat` out of each world and serves a world's icon
//!     and path;
//!   * `resourcepack.rs` and `screenshots.rs` list the other two content kinds;
//!   * `favorites.rs` is the shared favorites file.
//!
//! Two design notes, both deliberate:
//!
//!   * `worldmap.rs` renders a save's map **without a PNG round trip**.
//!     `render_map` hands the RGBA buffer straight back and the caller wraps it
//!     in a `SharedPixelBuffer`, so no encode, base64 or decode is involved.
//!   * The entry points take borrowed names (`&str`, `impl AsRef<Path>`) rather
//!     than owned strings; the favorites helpers and `remove_mod_files` are the
//!     exceptions, taking owned `String`s.
//!
//! [`content_counts`] backs the game view's content counts — the "n items"
//! labels on its preview rows.

use std::path::{Path, PathBuf};

use folder::DATA_LOCATION;

pub mod error;
pub mod favorites;
pub mod mods;
pub mod resourcepack;
pub mod saves;
pub mod screenshots;
pub mod worldmap;

/// How many items of each kind the current instance contains.
#[derive(Clone, Copy, Default)]
pub struct ContentCounts {
    pub saves: u32,
    pub mods: u32,
    pub resourcepacks: u32,
    pub screenshots: u32,
}

impl ContentCounts {
    /// Whether every category is empty.
    pub fn is_empty(&self) -> bool {
        self.saves == 0 && self.mods == 0 && self.resourcepacks == 0 && self.screenshots == 0
    }
}

/// Scans the local content of an instance.
pub fn content_counts(instance_id: &str) -> ContentCounts {
    let root = DATA_LOCATION.get_instance_root(instance_id);
    ContentCounts {
        saves: count_saves(&root.join("saves")),
        mods: count_mods(&root.join("mods")),
        resourcepacks: count_entries(&root.join("resourcepacks")),
        screenshots: count_images(&root.join("screenshots")),
    }
}

/// A save is a directory that contains a `level.dat` (and usually `region/`).
fn count_saves(dir: &Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.path().join("level.dat").is_file())
        .count() as u32
}

/// Mods are `*.jar` files (including disabled `*.jar.disabled`).
fn count_mods(dir: &Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains(".jar")
        })
        .count() as u32
}

/// Resource packs are either directories or `.zip` archives.
fn count_entries(dir: &Path) -> u32 {
    std::fs::read_dir(dir)
        .map(|entries| entries.flatten().count() as u32)
        .unwrap_or(0)
}

/// Screenshots are image files.
fn count_images(dir: &Path) -> u32 {
    const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    IMAGE_EXTENSIONS
                        .iter()
                        .any(|allowed| ext.eq_ignore_ascii_case(allowed))
                })
        })
        .count() as u32
}

/// Absolute path of an instance directory (helper for the app layer).
pub fn instance_root(instance_id: &str) -> PathBuf {
    DATA_LOCATION.get_instance_root(instance_id)
}
