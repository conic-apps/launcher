// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! `crates/modrinth/src/error.rs`, minus the Tauri IPC boundary.
//!
//! The original derives `Serialize` so the error can cross `invoke()`; the
//! attribute and the `serde_with::DisplayFromStr` shims are kept so the shape
//! the frontend sees (`{"kind": …, "message": …}`, with `{"kind": …}` alone for
//! the unit variant) does not change.

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
    Network(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::Error,
    ),
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),
    #[error("{0}")]
    ChecksumMissmatch(String),

    #[error(transparent)]
    UrlParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        url::ParseError,
    ),

    #[error("Chunk length mismatch")]
    ChunkLengthMismatch,

    #[error(transparent)]
    JsonParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        serde_json::Error,
    ),

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tokio::task::JoinError,
    ),
}

impl From<slint_download::Error> for Error {
    fn from(value: slint_download::Error) -> Self {
        match value {
            slint_download::Error::Io(error) => Self::Io(error),
            slint_download::Error::ChecksumMissmatch(error) => Self::ChecksumMissmatch(error),
            slint_download::Error::Network(error) => Self::Network(error),
            slint_download::Error::UrlParse(error) => Self::UrlParse(error),
            slint_download::Error::ChunkLengthMismatch => Self::ChunkLengthMismatch,
            slint_download::Error::Aborted(error) => Self::Aborted(error),
        }
    }
}
