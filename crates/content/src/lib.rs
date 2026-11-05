// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance's local content: offline readers for mods, resource packs,
//! saves, screenshots and favorites, plus `remote/` for resolving installed
//! archives against Modrinth and CurseForge.
//!
//! There is deliberately no cheap content-count scan here. Only the parsers
//! agree on what counts (jar-in-jar mods, jars that are not mods), so callers
//! parse once and count the result; see `ui/overlays/content/cache.rs`.

use std::path::PathBuf;

use storage::LOCATIONS;

pub mod error;
pub mod favorites;
mod index;
pub mod kind;
pub mod mods;
pub mod remote;
pub mod resourcepack;
pub mod saves;
pub mod screenshots;
pub mod worldmap;

pub fn instance_root(instance_id: &str) -> PathBuf {
    LOCATIONS.instances.get_instance_root(instance_id)
}
