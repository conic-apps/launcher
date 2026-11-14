// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world map's tile sources: which [`TileSource`] a [`WorldId`] gets. Only a
//! save is supported so far, rendered from its region files.

use std::sync::{Arc, OnceLock};

use tilemap::{TileSource, WorldId};

/// The save source, shared process-wide so its world cache survives a switch
/// between saves.
static SAVES: OnceLock<Arc<dyn TileSource>> = OnceLock::new();

fn save_source() -> Arc<dyn TileSource> {
    Arc::clone(SAVES.get_or_init(|| Arc::new(content::worldmap::LocalSaveSource::default())))
}

/// The tile source for `world`. A seed world falls back to the save source,
/// which rejects it rather than rendering the wrong thing.
pub(crate) fn source_for(_world: &WorldId) -> Arc<dyn TileSource> {
    save_source()
}
