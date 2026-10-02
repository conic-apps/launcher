// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world-map tile port.
//!
//! A map is drawn from tiles, and a tile comes from somewhere: a save's region
//! files (`content`), or a seed rendered by a fixed algorithm (a future
//! `seedmap`). Those are different domains, so the port they share lives here
//! and neither depends on the other — [`TileSource`] is implemented by both, and
//! the app picks one for the [`WorldId`] it is showing.
//!
//! Everything crossing the port is plain data: a rendered tile is an RGBA
//! buffer, not a UI image, so it can cross a thread.

/// Which world a tile belongs to, and how it is identified. The same value keys
/// a cache, so a different world means every tile is re-rendered.
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

/// A tile's position in the tile grid. `tile * size` is its top-left block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub x: i32,
    pub z: i32,
}

/// The toggles every tile of a world is rendered with.
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

/// One tile to render: a `size`×`size` square of blocks centred on `center`.
#[derive(Debug, Clone)]
pub struct TileRequest {
    pub world: WorldId,
    pub tile: TileKey,
    pub center: (i32, i32),
    pub size: u32,
    pub options: RenderOptions,
}

/// A rendered tile: an RGBA bitmap, one block per pixel, row-major.
#[derive(Debug, Clone)]
pub struct TileImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Why a tile could not be rendered.
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// The output port a world-map source implements.
///
/// `Send + Sync` because one source is shared across the app and its `render` is
/// called from a blocking worker per tile.
pub trait TileSource: Send + Sync {
    fn render(&self, request: &TileRequest) -> Result<TileImage, Error>;
}
