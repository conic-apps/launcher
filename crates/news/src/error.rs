// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Errors for the launcher-content client.
//!
//! Unlike the Modrinth and CurseForge clients, nothing here is handed to the
//! frontend: a failed news fetch only makes the panel show its empty state, and
//! the reason goes to the log. So the error stays a plain `thiserror` enum and
//! carries no `Serialize` shape.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Network(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
