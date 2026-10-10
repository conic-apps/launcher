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
//! There is deliberately no cheap "how many mods does this instance have" scan
//! here. A count is a second, disagreeing answer to a question the readers above
//! already answer exactly — `parse_mods` skips a jar that is not a mod and
//! yields one entry per jar-in-jar mod, where counting files counts neither —
//! and it only looks cheaper because it opens nothing. The caller that shows the
//! same content in two places (the game view's summary and the content overlay)
//! parses it once and counts that; see `ui/overlays/content/cache.rs`.

use std::path::PathBuf;

use storage::LOCATIONS;

pub mod error;
pub mod favorites;
pub mod mods;
pub mod resourcepack;
pub mod saves;
pub mod screenshots;
pub mod worldmap;

/// Absolute path of an instance directory.
pub fn instance_root(instance_id: &str) -> PathBuf {
    LOCATIONS.instances.get_instance_root(instance_id)
}
