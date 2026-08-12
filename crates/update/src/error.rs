// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::result;

use serde::Serialize;
use serde_with::serde_as;
use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

#[serde_as]
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum Error {
    #[error(transparent)]
    Updater(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tauri_plugin_updater::Error,
    ),

    #[error(transparent)]
    UrlParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        url::ParseError,
    ),

    #[error("No update available")]
    NoUpdateAvailable,

    #[error("Update task was cancelled")]
    Cancelled,

    #[error(transparent)]
    Join(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tokio::task::JoinError,
    ),
}
