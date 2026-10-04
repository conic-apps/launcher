// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! What can go wrong while checking, downloading or applying an update.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("the update request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("the update server returned an unreadable manifest: {0}")]
    Json(#[from] serde_json::Error),

    #[error("the update URL is not valid: {0}")]
    Url(#[from] url::ParseError),

    #[error("the update endpoint cannot be built: {0}")]
    Endpoint(#[from] shared::UrlExtError),

    /// The manifest's version, or the running one, is not a SemVer. A local
    /// build is allowed to carry a version the server cannot order, so this is
    /// reported and treated as "nothing to do" rather than as a failure.
    #[error("could not compare versions: {0}")]
    Version(String),

    /// The bundle did not verify against the embedded public key. Treated as a
    /// hard stop: a bundle that fails this is the one case that must never be
    /// applied.
    #[error("the update signature did not verify")]
    Signature,

    /// The server declared a SHA-256 that the downloaded bytes do not match.
    #[error("the update checksum did not match")]
    Checksum,

    #[error("the update server answered with HTTP {0}")]
    Status(u16),

    #[error("the running executable's path is unavailable")]
    NoCurrentExe,

    /// A package-manager install cannot update itself; the settings page greys
    /// the toggle out instead of ever reaching here.
    #[error("this install is managed by a package manager")]
    PackageManaged,

    #[error("this update form is not supported yet: {0}")]
    Unsupported(&'static str),
}
