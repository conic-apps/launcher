// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The frontend shows the error as one message, so every variant carries the
//! step that failed; the underlying crate error is the source where there is
//! one.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not read the zip archive {path}: {source}")]
    Zip {
        path: String,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("{0}")]
    Manifest(String),
    #[error("{0}")]
    Loader(String),
    #[error("could not resolve the modpack's files: {0}")]
    Source(String),
    #[error("the new instance could not be read back")]
    MissingInstance,
    #[error("{0}")]
    Instance(String),
    #[error("could not install Minecraft and the loader: {0}")]
    Install(#[from] install::Error),
    #[error("could not download the modpack's files: {0}")]
    Download(#[from] download::Error),
    /// A resolved file would be written outside the instance root.
    #[error("the modpack file path is not safe: {0}")]
    UnsafePath(String),
    #[error("could not write {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not copy the modpack overrides: {0}")]
    Overrides(std::io::Error),
    #[error("a background task failed: {0}")]
    Task(String),
}
