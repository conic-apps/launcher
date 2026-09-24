// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Lightweight, Tauri-free mirror of `crates/content` for the Slint app.
//!
//! The Game view only needs the *counts* of the current instance's local
//! content to lay out its preview rows; the heavy preview machinery (NBT level
//! parsing, save icons, mod metadata, world-map rendering) is not ported here
//! yet. Everything is scanned directly from the instance directory, matching
//! the layout the Tauri `content` crate reads.

use std::path::{Path, PathBuf};

use slint_folder::DATA_LOCATION;

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
