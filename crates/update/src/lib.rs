// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher's self-updater.
//!
//! The protocol is the one served by `conic-apps/conic-updater-server`: a
//! dynamic endpoint that answers `204`/JSON (see [`manifest`]), a minisign
//! signature over every bundle (see [`staged`]), and a per-platform apply step
//! that runs as the process exits (see [`apply`]).
//!
//! What this crate deliberately does not own: *when* to check, how to surface
//! progress, and whether the user opted in. Those are the use case's and the
//! interface's, so this crate only exposes the mechanism.

pub mod apply;
pub mod error;
pub mod install;
pub mod manifest;
pub mod staged;

pub use apply::apply_pending;
pub use error::{Error, Result};
pub use install::{InstallForm, InstallKind, detect};
pub use manifest::{Channel, UpdateInfo, check};
pub use staged::{DownloadProgress, Staged, clear_pending, download_and_stage, read_pending};
