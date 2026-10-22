// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The pack-format layer.
//!
//! [`inspect`] detects the format from the dropped path and asks that format to
//! read it; [`resolve`] dispatches the same way. Adding Modrinth, MultiMC/Prism,
//! MCBBS or HMCL is a module next to `curseforge`, a `Format`/`FormatInspection`
//! arm, and its marker in `UNSUPPORTED` until it is implemented.
//!
//! The installer never sees this module: it consumes the common [`Plan`].

mod curseforge;

use std::path::Path;

use crate::error::{Error, Result};
use crate::overrides::Overrides;
use crate::{Inspection, Plan, ResolvedFile, UnavailableFile};

/// The format-specific half of an [`Inspection`].
#[derive(Clone)]
pub(crate) enum FormatInspection {
    CurseForge(curseforge::Inspection),
}

/// What a format resolves a pack into; the common [`Plan`] adds the pack
/// identity around it.
pub(crate) struct ResolvedPack {
    pub(crate) files: Vec<ResolvedFile>,
    pub(crate) unavailable: Vec<UnavailableFile>,
    pub(crate) overrides: Vec<Overrides>,
}

#[derive(Debug)]
enum Format {
    CurseForge,
}

/// The manifest file CurseForge packs carry.
const CURSEFORGE_MARKER: &str = "manifest.json";

/// Formats this launcher can recognise but not import yet.
///
/// The marker is the format's own manifest at the pack root; a directory is
/// searched one level down as well, because a pack is often unzipped into one
/// folder. Move a format out of this table when its module lands.
const UNSUPPORTED: &[(&str, &[&str])] = &[
    ("Modrinth .mrpack", &["modrinth.index.json"]),
    ("Prism/MultiMC instance", &["mmc-pack.json", "instance.cfg"]),
    ("MCBBS modpack", &["mcbbs.packmeta"]),
    ("HMCL modpack", &["modpack.json"]),
];

pub(crate) fn inspect(path: &Path) -> Result<Inspection> {
    match detect(path)? {
        Format::CurseForge => curseforge::inspect(path),
    }
}

pub(crate) async fn resolve(inspection: Inspection) -> Result<Plan> {
    let Inspection {
        name,
        minecraft,
        loader_type,
        loader_version,
        format,
    } = inspection;
    let resolved = match format {
        FormatInspection::CurseForge(inspection) => curseforge::resolve(inspection).await?,
    };
    Ok(Plan {
        name,
        minecraft,
        loader_type,
        loader_version,
        unavailable: resolved.unavailable,
        files: resolved.files,
        overrides: resolved.overrides,
    })
}

fn detect(path: &Path) -> Result<Format> {
    let markers = markers(path)?;
    if markers.iter().any(|marker| marker == CURSEFORGE_MARKER) {
        return Ok(Format::CurseForge);
    }
    for (format, names) in UNSUPPORTED {
        if names
            .iter()
            .any(|name| markers.iter().any(|marker| marker == name))
        {
            return Err(Error::Manifest(format!(
                "this is a {format}; only CurseForge packs are supported for now"
            )));
        }
    }
    Err(Error::Manifest(format!(
        "{} is not a CurseForge modpack",
        path.display()
    )))
}

/// The file names a dropped pack exposes at its manifest level.
fn markers(path: &Path) -> Result<Vec<String>> {
    if path.is_dir() {
        let mut markers = Vec::new();
        let entries = std::fs::read_dir(path).map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| Error::Read {
                path: path.display().to_string(),
                source,
            })?;
            let candidate = entry.path();
            if candidate.is_file() {
                if let Some(name) = candidate.file_name().and_then(|name| name.to_str()) {
                    markers.push(name.to_string());
                }
            } else if candidate.is_dir()
                && let Ok(entries) = std::fs::read_dir(&candidate)
            {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        markers.push(name.to_string());
                    }
                }
            }
        }
        Ok(markers)
    } else {
        let file = std::fs::File::open(path).map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        let archive = zip::ZipArchive::new(file).map_err(|source| Error::Zip {
            path: path.display().to_string(),
            source,
        })?;
        let markers = (0..archive.len())
            .filter_map(|index| archive.name_for_index(index))
            .filter_map(|name| name.rsplit('/').next())
            .map(str::to_owned)
            .collect();
        Ok(markers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_curseforge_directory() {
        let directory =
            std::env::temp_dir().join(format!("conic-format-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        std::fs::write(directory.join("manifest.json"), "{}").expect("a manifest");
        assert!(matches!(detect(&directory), Ok(Format::CurseForge)));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn reports_a_known_unsupported_format() {
        let directory =
            std::env::temp_dir().join(format!("conic-format-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        std::fs::write(directory.join("modrinth.index.json"), "{}").expect("a manifest");
        let error = detect(&directory).expect_err("an unsupported format");
        assert!(error.to_string().contains("Modrinth"), "{error}");
        std::fs::remove_dir_all(&directory).ok();
    }
}
