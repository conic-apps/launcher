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
pub use install::{Progress, create_instance, install};

use std::path::{Path, PathBuf};

use instance::ModLoaderType;

/// What a dropped pack is, before its file list is resolved.
#[derive(Clone)]
pub struct Inspection {
    /// The pack's display name, which seeds the instance name.
    name: String,
    minecraft: String,
    loader_type: Option<ModLoaderType>,
    loader_version: Option<String>,
    format: format::FormatInspection,
}

impl Inspection {
    /// The pack's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The Minecraft version the pack pins.
    pub fn minecraft(&self) -> &str {
        &self.minecraft
    }

    /// The loader the pack pins, if any.
    pub fn loader_type(&self) -> Option<&ModLoaderType> {
        self.loader_type.as_ref()
    }

    /// The loader version the pack pins, if any.
    pub fn loader_version(&self) -> Option<&str> {
        self.loader_version.as_deref()
    }
}

/// A fully resolved pack, ready to install.
pub struct Plan {
    /// The pack's display name, which seeds the instance id.
    name: String,
    /// The files that could not be resolved, in manifest order.
    unavailable: Vec<UnavailableFile>,
    minecraft: String,
    loader_type: Option<ModLoaderType>,
    loader_version: Option<String>,
    files: Vec<ResolvedFile>,
    overrides: Vec<overrides::Overrides>,
}

impl Plan {
    /// The files that could not be resolved, in manifest order.
    pub fn unavailable(&self) -> &[UnavailableFile] {
        &self.unavailable
    }
}

/// A manifest file that could not be turned into a download.
#[derive(Debug, Clone)]
pub struct UnavailableFile {
    /// The file name when the source knew it, otherwise a source-specific label.
    file_name: String,
    /// The source's own reference for the file, if it has one.
    reference: String,
    required: bool,
    reason: String,
}

impl UnavailableFile {
    /// The file name when the source knew it, otherwise a source-specific label.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The source's own reference for the file.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Whether the pack requires this file.
    pub fn required(&self) -> bool {
        self.required
    }

    /// Why the file could not be resolved.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// One resolved download and where it goes under the instance root.
pub(crate) struct ResolvedFile {
    pub(crate) path: PathBuf,
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
