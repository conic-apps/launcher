// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The `forge` module contains functionality related to Forge installation and version management.
//!
//! This module re-exports the `install` function from the `install` submodule,
//! and exposes the `version_list` submodule for managing Forge versions.

pub mod install;
pub mod version_list;

pub use install::install;
