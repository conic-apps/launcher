// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;
use serde_with::serde_as;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[serde_as]
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum Error {
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),

    #[error(transparent)]
    ToStr(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::header::ToStrError,
    ),

    #[error(transparent)]
    Network(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::Error,
    ),

    #[error("The library file could not be downloaded from all sources")]
    AllSourceFailed,

    #[error(transparent)]
    LibLoader(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        libloader::libloading::Error,
    ),

    #[error("The library file is broken")]
    ChecksumMismatch,

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        futures::future::Aborted,
    ),
}
