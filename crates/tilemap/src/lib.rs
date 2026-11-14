// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world-map tile port. [`TileSource`] is what a save's region files and a
//! seed-rendered map would share, and the app picks an implementation for the
//! [`WorldId`] it is showing. Everything crossing it is plain data, so a rendered
//! tile can cross a thread.

/// The same value keys a cache, so a different world means every tile is
/// re-rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldId {
    /// A save folder inside an instance.
    Save {
        instance_id: String,
        /// The save's folder name, the world's identifier in the saves list.
        folder: String,
        /// A namespaced id, or the overworld.
        dimension: String,
    },
    /// A world generated from a seed rather than read from disk.
    Seed { seed: i64, dimension: String },
}

/// `tile * size` is its top-left block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    pub water: bool,
    pub shading: bool,
    pub altitude_shading: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            water: true,
            shading: true,
            altitude_shading: true,
        }
    }
}

/// A `size`×`size` square of blocks centred on `center`.
#[derive(Debug, Clone)]
pub struct TileRequest {
    pub world: WorldId,
    pub tile: TileKey,
    pub center: (i32, i32),
    pub size: u32,
    pub options: RenderOptions,
}

/// An RGBA bitmap, one block per pixel, row-major.
#[derive(Debug, Clone)]
pub struct TileImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// `Send + Sync` because one source is shared across the app and `render` is
/// called from a blocking worker per tile.
pub trait TileSource: Send + Sync {
    fn render(&self, request: &TileRequest) -> Result<TileImage, Error>;
}
