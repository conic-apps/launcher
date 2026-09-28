// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/content`: the instance's local content.
//!
//! The original is a Tauri plugin whose fifteen commands are thin wrappers —
//! none of them takes `State`, `AppHandle` or a `Channel` — so the mirror drops
//! the command layer and keeps the modules themselves, which can then be
//! diffed against the original file for file:
//!
//!   * `mods/` reads the mods folder and resolves each jar's metadata, offline
//!     from the archive itself and online through Modrinth and CurseForge;
//!   * `saves/` reads `level.dat` out of each world and serves a world's icon
//!     and path;
//!   * `resourcepack.rs` and `screenshots.rs` list the other two content kinds;
//!   * `favorites.rs` is the shared favorites file.
//!
//! Two deviations, both deliberate:
//!
//!   * `worldmap.rs` is **not** mirrored. The saves overlay's card expansion
//!     draws a live world map through the external `conic-worldmap` crate; that
//!     is a feature of its own and is left for a later pass (see the Slint
//!     README's Known issues), so the error variants that existed only for it
//!     (`WorldMap`, `WorldMapTask`, `WorldMapPng`) are gone too.
//!   * The entry points take `&str` where the original took `String`. The
//!     owned strings were what Tauri's IPC deserialization produced; nothing
//!     here needs them.
//!
//! [`content_counts`] is not part of the original — it is what the game view's
//! preview rows read for their "n items" labels.

use std::path::{Path, PathBuf};

use slint_folder::DATA_LOCATION;

pub mod error;
pub mod favorites;
pub mod mods;
pub mod resourcepack;
pub mod saves;
pub mod screenshots;

/// How many items of each kind the current instance contains.
#[derive(Clone, Copy, Default)]
pub struct ContentCounts {
    pub saves: u32,
    pub mods: u32,
    pub resourcepacks: u32,
    pub screenshots: u32,
}

impl ContentCounts {
    /// Whether every category is empty (used to disable the preview rows).
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
