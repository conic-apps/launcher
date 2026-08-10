// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Mods parser. It supports `forge`, `neoforge`, `fabric`, `quilt` and `liteloader`.
//!
//! [`parse_mod`] resolves every mod contained in a single archive (including
//! jar-in-jar dependencies), while [`parse_folder`] resolves all mods inside a
//! directory. Files that can't be recognized as a mod are skipped, and files
//! that look like a mod but fail to parse are kept as a fallback entry with
//! [`ModLoader::Unknown`] so they don't silently disappear from the list.
//!
//! # Example
//!
//! Resolve all mods in the folder:
//!
//! ```no_run
//! use content::mods::parse_folder;
//!
//! let result = parse_folder("/path/to/mods").unwrap();
//! println!("{:#?}", result);
//! ```

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use crate::mods::remote::RemoteModPlatform;
use base64::Engine;
use base64::engine::general_purpose;
use serde_json::Value;
use zip::ZipArchive;

use crate::error::{Error, Result};

pub mod fabric;
pub mod forge;
pub mod liteloader;
pub mod quilt;
pub mod remote;

/// The mod loader a resolved mod is targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    Forge,
    NeoForge,
    Fabric,
    Quilt,
    LiteLoader,
    Unknown,
}

/// A mod icon as declared in `fabric.mod.json` / `quilt.mod.json`. It can be a
/// single path, or a `{resolution: path}` map.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum ModIcon {
    /// A single path to a PNG file inside the archive.
    Path(String),
    /// A map of resolutions to paths. The largest resolution is preferred.
    Sizes(HashMap<String, String>),
}

impl ModIcon {
    /// The path to read from the archive, preferring the largest resolution.
    pub fn path(&self) -> &str {
        match self {
            ModIcon::Path(p) => p,
            ModIcon::Sizes(sizes) => sizes
                .iter()
                .max_by_key(|(size, _)| size.parse::<u64>().unwrap_or(0))
                .map(|(_, path)| path.as_str())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedMod {
    /// The mod file this entry was resolved from. Filled in by the path-taking
    /// entry points ([`parse_mod`], [`parse_folder`] and the remote lookup);
    /// never empty in a result handed back to the frontend.
    pub path: PathBuf,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub depends: ResolvedDepends,
    pub authors: Vec<ResolvedAuthorInfo>,
    pub license: Option<Vec<String>>,
    /// The mod icon encoded as a `data:image/png;base64,` data URL, or the icon
    /// URL returned by an online lookup.
    pub icon: Option<String>,
    pub loader: ModLoader,
    /// Whether the mod is disabled, i.e. its file name carries the
    /// `.jar.disabled` suffix and the loader will not load it.
    pub disabled: bool,
    /// Whether the mod was resolved from a jar embedded inside another mod
    /// (jar-in-jar dependency). The frontend can hide these to show only the
    /// main jars. Defaults to `false` so cached entries without the field
    /// still deserialize.
    #[serde(default)]
    pub embedded: bool,
    /// Where the mod was resolved from online (`modrinth` / `curseforge`),
    /// when an online lookup matched this file.
    pub source: Option<RemoteModPlatform>,
    /// The project/mod id on the source platform.
    pub source_id: Option<String>,
    /// The version/file id on the source platform.
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedDepends {
    pub minecraft: Option<Value>,
    pub java: Option<Value>,
    pub mod_loader: Option<Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedAuthorInfo {
    pub name: String,
    pub contact: Option<HashMap<String, String>>,
}

impl ResolvedMod {
    /// Fallback entry for a file that looks like a mod but could not be parsed,
    /// so the user still knows "this is at least a mod file".
    pub(crate) fn unrecognized(path: &Path) -> Self {
        ResolvedMod {
            path: path.to_path_buf(),
            name: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            description: None,
            version: None,
            depends: ResolvedDepends {
                minecraft: None,
                java: None,
                mod_loader: None,
            },
            authors: Vec::new(),
            license: None,
            icon: None,
            loader: ModLoader::Unknown,
            disabled: false,
            embedded: false,
            source: None,
            source_id: None,
            version_id: None,
        }
    }
}

/// Mods parser. It supports `forge`, `neoforge`, `fabric`, `quilt` and `liteloader`.
///
/// It will parse the mod using a parser that is suitable for the mod, and returns
/// every mod found inside the archive, including jar-in-jar dependencies.
///
/// - `Ok(..)`: the file was recognized as a mod (or several).
/// - `Err(Error::NotAModFile)`: the file is not a mod at all.
/// - `Err(_)`: the file is a mod but its metadata could not be parsed.
pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<Vec<ResolvedMod>> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(|_| Error::NotAModFile)?;
    let mut mods = parse_mod_archive(&mut archive)?;
    if is_disabled_file(path) {
        for mod_info in &mut mods {
            mod_info.disabled = true;
        }
    }
    for mod_info in &mut mods {
        mod_info.path = path.to_path_buf();
    }
    Ok(mods)
}

/// A per-loader archive parser function.
pub type LoaderParser<R> = fn(&mut ZipArchive<R>) -> Result<Vec<ResolvedMod>>;

/// Parse a mod archive, trying each supported loader in order until one of them
/// recognizes the file. Nested jars are resolved recursively.
pub fn parse_mod_archive<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<ResolvedMod>> {
    let loaders: [LoaderParser<R>; 4] = [
        quilt::parse_mod_archive,
        fabric::parse_mod_archive,
        forge::parse_mod_archive,
        liteloader::parse_mod_archive,
    ];
    for loader in loaders {
        match loader(archive) {
            Ok(mods) => return Ok(mods),
            Err(Error::NotAModFile) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(Error::NotAModFile)
}

pub fn parse_folder<S: AsRef<OsStr> + ?Sized>(folder: &S) -> Result<Vec<ResolvedMod>> {
    let folder = Path::new(folder);
    let entries = folder.read_dir()?;
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        match parse_mod(&path) {
            Ok(mods) => result.extend(mods),
            Err(Error::NotAModFile) => continue,
            Err(e) => {
                log::warn!("Failed to parse mod {:?}: {e}", path);
                let mut unrecognized = ResolvedMod::unrecognized(&path);
                unrecognized.disabled = is_disabled_file(&path);
                result.push(unrecognized);
            }
        }
    }
    Ok(result)
}

/// Whether a mod file is disabled, i.e. its file name carries the `.jar.disabled`
/// (or `.jar.disable`) suffix that launchers use to keep a mod out of the mods folder.
pub fn is_disabled_file<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".jar.disabled") || name.ends_with(".jar.disable"))
        .unwrap_or(false)
}

/// Strip a UTF-8 BOM from the start of a metadata file.
pub(crate) fn strip_bom(s: &str) -> &str {
    s.trim_start_matches('\u{feff}')
}

/// Sanitize mod metadata JSON before parsing. Some mods embed raw newlines in
/// strings (invalid JSON), so like xmcl we remove every line break first.
pub(crate) fn sanitize_json(s: &str) -> String {
    strip_bom(s).replace(['\n', '\r'], "")
}

/// Read an entry from the archive and encode it as a base64 PNG data URL.
pub(crate) fn read_icon<R: Read + Seek>(archive: &mut ZipArchive<R>, path: &str) -> Option<String> {
    let mut buf = Vec::new();
    if let Ok(mut file) = archive.by_name(path)
        && file.read_to_end(&mut buf).is_ok()
    {
        return Some(format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD_NO_PAD.encode(buf)
        ));
    }
    None
}

/// Read a whole entry from the archive as bytes. Used to open nested jars.
pub(crate) fn read_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    if let Ok(mut file) = archive.by_name(path)
        && file.read_to_end(&mut buf).is_ok()
    {
        return Some(buf);
    }
    None
}

/// Open a nested jar (given as raw bytes) as a zip archive.
pub(crate) fn open_nested_jar<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Option<ZipArchive<std::io::Cursor<Vec<u8>>>> {
    let bytes = read_entry(archive, path)?;
    ZipArchive::new(std::io::Cursor::new(bytes)).ok()
}
