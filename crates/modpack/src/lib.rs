// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! CurseForge modpack import.
//!
//! The crate is split the way a pack is:
//!
//! - `format` detects the pack format from the dropped path and turns it into
//!   an [`Inspection`] and then a [`Plan`]. CurseForge is the only format today;
//!   Modrinth, MultiMC/Prism, MCBBS and HMCL each get a module there and an arm
//!   in the detection/dispatch, while the installer stays untouched.
//! - `overrides` extracts the pack's file trees (the format decides which trees
//!   they are).
//! - `install` creates the instance, installs Minecraft and the loader through
//!   the `install` crate, downloads the plan's files and applies the overrides.
//!
//! Progress is reported through a [`shared::Sink`] of [`Progress`], so the
//! frontend can drive its own install screen.

mod error;
mod format;
mod install;
mod overrides;

pub use error::{Error, Result};
pub use install::{Progress, ProgressSink, create_instance, install};

use std::path::Path;

use instance::ModLoaderType;

/// What a dropped pack is, before its file list is resolved.
#[derive(Clone)]
pub struct Inspection {
    /// The pack's display name, which seeds the instance name.
    pub name: String,
    pub minecraft: String,
    pub loader_type: Option<ModLoaderType>,
    pub loader_version: Option<String>,
    pub(crate) format: format::FormatInspection,
}

/// A fully resolved pack, ready to install.
pub struct Plan {
    /// The pack's display name, which seeds the instance id.
    pub name: String,
    /// The files that could not be resolved, in manifest order.
    pub unavailable: Vec<UnavailableFile>,
    pub(crate) minecraft: String,
    pub(crate) loader_type: Option<ModLoaderType>,
    pub(crate) loader_version: Option<String>,
    pub(crate) files: Vec<ResolvedFile>,
    pub(crate) overrides: Vec<overrides::Overrides>,
}

/// A manifest file that could not be turned into a download.
#[derive(Debug, Clone)]
pub struct UnavailableFile {
    /// The file name when the API knew it, otherwise a `project/file` label.
    pub file_name: String,
    pub project_id: i64,
    pub file_id: i64,
    pub required: bool,
    pub reason: String,
}

/// One resolved download, with the file name the source chose.
pub(crate) struct ResolvedFile {
    pub(crate) file_name: String,
    pub(crate) url: String,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) sha1: Option<String>,
}

/// Reads the manifest of a dropped pack. No network, no instance.
pub fn inspect(path: &Path) -> Result<Inspection> {
    format::inspect(path)
}

/// Resolves the manifest's file pairs into downloads.
pub async fn resolve(inspection: Inspection) -> Result<Plan> {
    format::resolve(inspection).await
}
