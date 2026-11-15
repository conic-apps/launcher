// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The files a dropped pack exposes, whether it is a zip or an already
//! extracted folder, and the pack root inside it.
//!
//! A pack is often unzipped into one folder, so the root is not the dropped
//! path itself: it is the shallowest folder that holds one of the format
//! markers. Resolving it once here keeps every format from guessing, and a
//! zip's entries and a folder's contents then read the same way.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::overrides::Overrides;

/// A pack opened at its root.
pub(crate) enum Source {
    Directory {
        base: PathBuf,
    },
    Archive {
        path: PathBuf,
        /// The entry prefix of the pack root, including its trailing `/`; empty
        /// when the root is the archive itself.
        base: String,
    },
}

impl Source {
    /// Opens `path` at the folder that holds one of `markers`.
    pub(crate) fn open(path: &Path, markers: &[&str]) -> Result<Self> {
        if path.is_dir() {
            open_directory(path, markers)
        } else {
            open_archive(path, markers)
        }
    }

    /// Every entry below the root, `/`-separated.
    pub(crate) fn names(&self) -> Result<Vec<String>> {
        match self {
            Self::Archive { path, base } => Ok(archive_names(path)?
                .into_iter()
                .filter_map(|name| name.strip_prefix(base.as_str()).map(str::to_owned))
                .collect()),
            Self::Directory { base } => {
                let mut names = Vec::new();
                walk(base, "", &mut names)?;
                Ok(names)
            }
        }
    }

    /// Whether the pack carries `relative` as a file.
    pub(crate) fn has_file(&self, relative: &str) -> Result<bool> {
        match self {
            Self::Directory { base } => Ok(base.join(relative).is_file()),
            Self::Archive { path, base } => {
                let file = std::fs::File::open(path).map_err(|source| Error::Read {
                    path: path.display().to_string(),
                    source,
                })?;
                let mut archive = zip::ZipArchive::new(file).map_err(|source| Error::Zip {
                    path: path.display().to_string(),
                    source,
                })?;
                Ok(archive.by_name(&format!("{base}{relative}")).is_ok())
            }
        }
    }

    /// Whether the pack carries the `prefix` tree.
    ///
    /// A tree is a folder, not a file, so it cannot be read off [`Source::names`]:
    /// a zip need not store its directory entries.
    pub(crate) fn has_tree(&self, prefix: &str) -> Result<bool> {
        let prefix = prefix.trim_matches('/');
        if prefix.is_empty() {
            return Ok(true);
        }
        match self {
            Self::Directory { base } => Ok(base.join(prefix).is_dir()),
            Self::Archive { path, base } => Ok(archive_names(path)?
                .iter()
                .any(|name| name.starts_with(&format!("{base}{prefix}/")))),
        }
    }

    /// Reads one entry of the pack, named relative to its root.
    pub(crate) fn read(&self, relative: &str) -> Result<Vec<u8>> {
        match self {
            Self::Directory { base } => {
                let path = base.join(relative);
                std::fs::read(&path).map_err(|source| Error::Read {
                    path: path.display().to_string(),
                    source,
                })
            }
            Self::Archive { path, base } => {
                let file = std::fs::File::open(path).map_err(|source| Error::Read {
                    path: path.display().to_string(),
                    source,
                })?;
                let mut archive = zip::ZipArchive::new(file).map_err(|source| Error::Zip {
                    path: path.display().to_string(),
                    source,
                })?;
                let entry = format!("{base}{relative}");
                let mut file = archive.by_name(&entry).map_err(|source| Error::Zip {
                    path: path.display().to_string(),
                    source,
                })?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(|source| Error::Read {
                    path: format!("{} ({entry})", path.display()),
                    source,
                })?;
                Ok(bytes)
            }
        }
    }

    /// The pack root, for error messages.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Directory { base } => base.display().to_string(),
            Self::Archive { path, base } => format!("{} ({base})", path.display()),
        }
    }

    /// The `prefix` tree of the pack, ready to copy over the instance root.
    pub(crate) fn overrides(&self, prefix: &str) -> Overrides {
        let prefix = prefix.trim_matches('/');
        match self {
            Self::Directory { base } => Overrides::directory(base.clone(), prefix.to_string()),
            Self::Archive { path, base } => {
                Overrides::archive(path.clone(), format!("{base}{prefix}/"))
            }
        }
    }
}

fn has_marker(directory: &Path, markers: &[&str]) -> bool {
    markers
        .iter()
        .any(|marker| directory.join(marker).is_file())
}

fn open_directory(path: &Path, markers: &[&str]) -> Result<Source> {
    if has_marker(path, markers) {
        return Ok(Source::Directory {
            base: path.to_path_buf(),
        });
    }
    let mut found = Vec::new();
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
        if candidate.is_dir() && has_marker(&candidate, markers) {
            found.push(candidate);
        }
    }
    match found.len() {
        0 => Err(not_a_pack(path)),
        1 => Ok(Source::Directory {
            base: found.remove(0),
        }),
        _ => Err(Error::Manifest(format!(
            "{} contains more than one modpack; drop the pack folder itself",
            path.display()
        ))),
    }
}

fn open_archive(path: &Path, markers: &[&str]) -> Result<Source> {
    let names = archive_names(path)?;
    // The marker folder, not the file: one format can put several markers in
    // the same folder (`mmc-pack.json` and `instance.cfg`), and that is one
    // pack. The shallowest folder wins, so a zip whose root holds a marker next
    // to a nested pack is the pack.
    let mut bases: Vec<(usize, String)> = names
        .iter()
        .filter_map(|name| {
            let last = name.rsplit('/').next()?;
            if !markers.contains(&last) {
                return None;
            }
            Some((
                name.matches('/').count(),
                name[..name.len() - last.len()].to_string(),
            ))
        })
        .collect();
    bases.sort();
    bases.dedup();
    let Some((depth, base)) = bases.first() else {
        return Err(not_a_pack(path));
    };
    if bases
        .iter()
        .filter(|(candidate_depth, _)| candidate_depth == depth)
        .count()
        > 1
    {
        return Err(Error::Manifest(format!(
            "{} contains more than one modpack; drop the pack archive itself",
            path.display()
        )));
    }
    Ok(Source::Archive {
        path: path.to_path_buf(),
        base: base.clone(),
    })
}

fn archive_names(path: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    let archive = zip::ZipArchive::new(file).map_err(|source| Error::Zip {
        path: path.display().to_string(),
        source,
    })?;
    Ok((0..archive.len())
        .filter_map(|index| archive.name_for_index(index).map(str::to_owned))
        .collect())
}

fn walk(base: &Path, relative: &str, names: &mut Vec<String>) -> Result<()> {
    let directory = if relative.is_empty() {
        base.to_path_buf()
    } else {
        base.join(relative)
    };
    let entries = std::fs::read_dir(&directory).map_err(|source| Error::Read {
        path: directory.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: directory.display().to_string(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        let child = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        if entry.path().is_dir() {
            walk(base, &child, names)?;
        } else {
            names.push(child);
        }
    }
    Ok(())
}

fn not_a_pack(path: &Path) -> Error {
    Error::Manifest(format!(
        "{} is not a modpack: it holds none of the CurseForge, Modrinth, Prism/MultiMC, \
         MCBBS or HMCL manifests",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKERS: &[&str] = &["manifest.json", "modrinth.index.json"];

    #[test]
    fn finds_the_pack_folder_inside_a_dropped_directory() {
        let directory =
            std::env::temp_dir().join(format!("conic-source-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(directory.join("My Pack/mods")).expect("a temporary directory");
        std::fs::write(directory.join("My Pack/manifest.json"), "{}").expect("a manifest");
        std::fs::write(directory.join("My Pack/mods/a.jar"), "jar").expect("a mod");
        let source = Source::open(&directory, MARKERS).expect("the pack root");
        let Source::Directory { base } = &source else {
            panic!("a directory pack");
        };
        assert_eq!(base, &directory.join("My Pack"));
        let names = source.names().expect("the entries");
        assert!(names.contains(&"manifest.json".to_string()));
        assert!(names.contains(&"mods/a.jar".to_string()));
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn reads_an_archive_through_its_root_prefix() {
        use std::io::Write as _;

        let archive =
            std::env::temp_dir().join(format!("conic-source-test-{}.zip", uuid::Uuid::new_v4()));
        {
            let file = std::fs::File::create(&archive).expect("a zip file");
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("pack/manifest.json", options)
                .expect("a manifest entry");
            zip.write_all(b"{}").expect("a manifest");
            zip.finish().expect("a finished zip");
        }
        let source = Source::open(&archive, MARKERS).expect("the pack root");
        assert!(matches!(&source, Source::Archive { base, .. } if base == "pack/"));
        assert_eq!(source.read("manifest.json").expect("the manifest"), b"{}");
        assert_eq!(
            source.names().expect("the entries"),
            vec!["manifest.json".to_string()]
        );
        std::fs::remove_file(&archive).ok();
    }

    #[test]
    fn rejects_a_dropped_folder_without_a_manifest() {
        let directory =
            std::env::temp_dir().join(format!("conic-source-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        assert!(Source::open(&directory, MARKERS).is_err());
        std::fs::remove_dir_all(&directory).ok();
    }
}
