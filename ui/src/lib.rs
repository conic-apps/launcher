// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Slint interface: the `.slint` tree, the Rust bindings generated from it,
//! and the assets the app reads as bytes.

pub mod assets;

#[allow(clippy::unwrap_used)]
mod generated {
    slint::include_modules!();
}

pub use generated::*;
