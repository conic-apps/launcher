// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher's self-updater.
//!
//! The protocol is served by `conic-apps/conic-updater-server`: a dynamic endpoint
//! answering `204`/JSON (see [`manifest`]), a minisign signature over every bundle
//! (see [`staged`]), and a per-platform apply step that runs as the process exits
//! (see [`apply`]). When to check and whether the user opted in are the caller's
//! concern; this crate only exposes the mechanism.

pub mod applied;
pub mod apply;
pub mod error;
pub mod install;
pub mod manifest;
pub mod staged;

pub use applied::{Applied, record as record_applied, take as take_applied};
pub use apply::apply_pending;
pub use error::{Error, Result};
pub use install::{InstallForm, InstallKind, detect};
pub use manifest::{Channel, UpdateInfo, check};
pub use staged::{DownloadProgress, Staged, clear_pending, download_and_stage, read_pending};
