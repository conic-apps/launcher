// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::result;

use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

/// Errors of the mirrored version-list capability.
///
/// The variants are the subset of `crates/install/src/error.rs` that the
/// version-list requests can produce; the install-task variants (zip, java
/// runtime, download, job abortion) arrive with the launch view.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error(transparent)]
    JsonParse(#[from] serde_json::Error),
    #[error("Version metadata not found in version manifest")]
    VersionMetadataNotfound,
}
