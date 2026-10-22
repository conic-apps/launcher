// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The errors a modpack import can produce.
//!
//! The frontend shows the error as one message, so every variant carries the
//! step that failed; the underlying crate error is the source where there is
//! one.

use thiserror::Error;

/// What a modpack import step returns.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything a modpack import can fail at.
#[derive(Debug, Error)]
pub enum Error {
    /// A manifest, an archive or a filesystem entry could not be read.
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// A pack archive could not be opened or decoded.
    #[error("could not read the zip archive {path}: {source}")]
    Zip {
        path: String,
        #[source]
        source: zip::result::ZipError,
    },
    /// The manifest is not a CurseForge manifest this importer understands.
    #[error("{0}")]
    Manifest(String),
    /// The manifest names a loader this launcher cannot install.
    #[error("{0}")]
    Loader(String),
    /// CurseForge file metadata could not be fetched.
    #[error("could not resolve the CurseForge files: {0}")]
    CurseForge(String),
    /// The created instance disappeared before the install could read it.
    #[error("the new instance could not be read back")]
    MissingInstance,
    /// Creating or listing an instance failed.
    #[error("{0}")]
    Instance(String),
    /// Installing Minecraft or the loader failed.
    #[error("could not install Minecraft and the loader: {0}")]
    Install(#[from] install::Error),
    /// Downloading the pack's files failed.
    #[error("could not download the modpack's files: {0}")]
    Download(#[from] download::Error),
    /// Writing an override file failed.
    #[error("could not write {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Copying the overrides failed.
    #[error("could not copy the modpack overrides: {0}")]
    Overrides(std::io::Error),
    /// A blocking filesystem task panicked or was cancelled.
    #[error("a background task failed: {0}")]
    Task(String),
}
