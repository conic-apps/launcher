// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Importing an instance from outside the launcher: a modpack archive in one of
//! the five pack formats, or another launcher's instance directory.
//!
//! [`inspect`] recognises a dropped path and [`resolve`] works out what it will
//! take to get it in; [`import()`] then creates the instance and fills it, so the
//! caller never sequences "create" before "install". Progress, including the new
//! instance id, is reported through a [`shared::Sink`] of [`Progress`].

mod error;
mod files;
mod import;
mod launcher;
mod loader;
mod pack;
mod settings;
mod source;

pub use error::{Error, Result};
pub use import::{Progress, import};

use std::path::{Path, PathBuf};

use instance::ModLoaderType;
use settings::Settings;

/// What a dropped path is, before its files are resolved.
#[derive(Clone)]
pub struct Import {
    /// The source's own name for it, which seeds the instance name.
    name: String,
    minecraft: String,
    loader_type: Option<ModLoaderType>,
    loader_version: Option<String>,
    settings: Settings,
    kind: ImportKind,
    source: Recognized,
}

impl Import {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn minecraft(&self) -> &str {
        &self.minecraft
    }

    pub fn loader_type(&self) -> Option<&ModLoaderType> {
        self.loader_type.as_ref()
    }

    pub fn loader_version(&self) -> Option<&str> {
        self.loader_version.as_deref()
    }

    /// What the import is: a modpack, or another launcher's instance.
    pub fn kind(&self) -> ImportKind {
        self.kind
    }
}

/// What an import brings in, which is what the frontend words it as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind {
    /// A modpack, whose files are looked up and downloaded.
    Modpack,
    /// Another launcher's instance, whose files are already on disk.
    Instance,
}

/// A fully resolved import, ready to bring in.
pub struct Plan {
    /// The source's display name, which seeds the instance id.
    name: String,
    /// The files that could not be resolved, in manifest order.
    unavailable: Vec<UnavailableFile>,
    minecraft: String,
    loader_type: Option<ModLoaderType>,
    loader_version: Option<String>,
    settings: Settings,
    files: Vec<ResolvedFile>,
    trees: Vec<files::FileTree>,
}

impl Plan {
    pub fn unavailable(&self) -> &[UnavailableFile] {
        &self.unavailable
    }
}

/// A source entry the launcher could not provide: a file it could not resolve,
/// or a component it does not install.
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
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub fn required(&self) -> bool {
        self.required
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// One resolved download and where it goes under the instance root.
pub(crate) struct ResolvedFile {
    pub(crate) path: PathBuf,
    pub(crate) url: String,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) checksum: download::Checksum,
}

/// The identity an import is recognised by; what a source reads before its
/// files are resolved.
pub(crate) struct Metadata {
    pub(crate) name: String,
    pub(crate) minecraft: String,
    pub(crate) loader_type: Option<ModLoaderType>,
    pub(crate) loader_version: Option<String>,
    pub(crate) settings: Settings,
}

/// What an import resolves into; the common [`Plan`] adds the identity around it.
pub(crate) struct Resolved {
    pub(crate) files: Vec<ResolvedFile>,
    pub(crate) unavailable: Vec<UnavailableFile>,
    pub(crate) trees: Vec<files::FileTree>,
}

/// Where an import's files come from: a pack format, or a launcher's instance
/// layout.
#[derive(Clone)]
pub(crate) enum Recognized {
    Pack(pack::Pack),
    Instance(launcher::Instance),
}

/// Reads what a dropped path is. No network, no instance.
pub fn inspect(path: &Path) -> Result<Import> {
    if let Some(source) = source::Source::try_open(path, pack::MARKERS)? {
        let (metadata, pack) = pack::inspect(path, &source)?;
        return Ok(Import {
            name: metadata.name,
            minecraft: metadata.minecraft,
            loader_type: metadata.loader_type,
            loader_version: metadata.loader_version,
            settings: metadata.settings,
            kind: pack.kind(),
            source: Recognized::Pack(pack),
        });
    }
    if let Some((metadata, instance)) = launcher::inspect(path)? {
        return Ok(Import {
            name: metadata.name,
            minecraft: metadata.minecraft,
            loader_type: metadata.loader_type,
            loader_version: metadata.loader_version,
            settings: metadata.settings,
            kind: ImportKind::Instance,
            source: Recognized::Instance(instance),
        });
    }
    Err(Error::Manifest(format!(
        "{} is not an instance this launcher can import: it holds none of the CurseForge, \
         Modrinth, Prism/MultiMC, MCBBS or HMCL manifests, nor another launcher's instance",
        path.display()
    )))
}

/// Resolves the files an import needs.
pub async fn resolve(candidate: Import) -> Result<Plan> {
    let Import {
        name,
        minecraft,
        loader_type,
        loader_version,
        settings,
        source,
        ..
    } = candidate;
    let resolved = match source {
        Recognized::Pack(pack) => pack::resolve(pack).await?,
        Recognized::Instance(instance) => instance.resolve(),
    };
    Ok(Plan {
        name,
        minecraft,
        loader_type,
        loader_version,
        settings,
        unavailable: resolved.unavailable,
        files: resolved.files,
        trees: resolved.trees,
    })
}
