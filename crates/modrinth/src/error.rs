// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
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
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        futures::future::Aborted,
    ),
}

impl From<download::Error> for Error {
    fn from(value: download::Error) -> Self {
        match value {
            download::Error::Io(error) => Self::Io(error),
            download::Error::ChecksumMissmatch(error) => Self::ChecksumMissmatch(error),
            download::Error::Network(error) => Self::Network(error),
            download::Error::UrlParse(error) => Self::UrlParse(error),
            download::Error::ChunkLengthMismatch => Self::ChunkLengthMismatch,
        }
    }
}
