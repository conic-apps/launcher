// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Slint interface: the `.slint` tree, the Rust bindings generated from it,
//! and the assets the app reads as bytes.
//!
//! The compiler turns the interface into a single very large Rust module. That
//! module belongs in a crate of its own so it is compiled when the interface
//! changes, rather than on every change to the app's Rust code.

pub mod assets;

// Slint-generated code uses `unwrap()` extensively, and `app` denies it for its
// own code; the allow stays scoped to this module.
#[allow(clippy::unwrap_used)]
mod generated {
    slint::include_modules!();
}

pub use generated::*;
