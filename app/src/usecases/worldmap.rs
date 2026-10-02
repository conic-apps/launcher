// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world map's tile sources.
//!
//! The map draws whatever [`TileSource`] it is handed; this module is what
//! decides which one a [`WorldId`] gets. A save is rendered from its region
//! files (`content`); a seed map would be a second arm here, backed by a crate
//! implementing the same port.

use std::sync::{Arc, OnceLock};

use tilemap::{TileSource, WorldId};

/// The save source, shared process-wide so the world cache it keeps survives a
/// switch between saves — the behaviour the map had when it held one cache.
static SAVES: OnceLock<Arc<dyn TileSource>> = OnceLock::new();

fn save_source() -> Arc<dyn TileSource> {
    Arc::clone(SAVES.get_or_init(|| Arc::new(content::worldmap::LocalSaveSource::default())))
}

/// The tile source for `world`.
///
/// There is no seed source yet, so a seed world falls back to the save source,
/// which rejects it rather than rendering the wrong thing.
pub(crate) fn source_for(_world: &WorldId) -> Arc<dyn TileSource> {
    save_source()
}
