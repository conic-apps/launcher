// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The pack file trees that are copied over the instance root.
//!
//! A format decides which trees it has (CurseForge's `overrides/`; Modrinth
//! would also contribute `client-overrides/`), and this module knows how to
//! extract a directory or an entry prefix from a zip. The install calls it
//! after the game is in place, so the pack's configs and `options.txt` win.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// One tree of files to copy over the instance root.
#[derive(Clone, Debug)]
pub(crate) enum Overrides {
    Directory {
        base: PathBuf,
        relative: String,
    },
    Archive {
        path: PathBuf,
        /// The entry prefix to strip before writing; e.g. `pack/overrides/`.
        prefix: String,
    },
}

/// Copies every tree over `instance_root`, in order.
pub(crate) async fn extract_all(overrides: &[Overrides], instance_root: &Path) -> Result<()> {
    for source in overrides {
        source.extract(instance_root).await?;
    }
    Ok(())
}

impl Overrides {
    async fn extract(&self, instance_root: &Path) -> Result<()> {
        match self {
            Self::Directory { base, relative } => {
                if relative.trim().is_empty() {
                    return Ok(());
                }
                let from = base.join(relative);
                if !from.is_dir() {
                    log::warn!("the modpack has no {} directory", from.display());
                    return Ok(());
                }
                let to = instance_root.to_path_buf();
                tokio::task::spawn_blocking(move || copy_recursively(&from, &to))
                    .await
                    .map_err(|error| Error::Task(error.to_string()))?
                    .map_err(Error::Overrides)
            }
            Self::Archive { path, prefix } => {
                if prefix.is_empty() {
                    return Ok(());
                }
                let path = path.clone();
                let prefix = prefix.clone();
                let to = instance_root.to_path_buf();
                tokio::task::spawn_blocking(move || extract_archive(&path, &prefix, &to))
                    .await
                    .map_err(|error| Error::Task(error.to_string()))?
            }
        }
    }
}

fn extract_archive(path: &Path, prefix: &str, to: &Path) -> Result<()> {
    let file = std::fs::File::open(path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|source| Error::Zip {
        path: path.display().to_string(),
        source,
    })?;
    let mut extracted = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|source| Error::Zip {
            path: path.display().to_string(),
            source,
        })?;
        // `enclosed_name` rejects `..` and absolute entries, so a pack cannot
        // write outside the instance root.
        let Some(name) = entry.enclosed_name() else {
            log::warn!(
                "skipping {} in {}: it is not an enclosed path",
                entry.name(),
                path.display()
            );
            continue;
        };
        let name = name.to_string_lossy().replace('\\', "/");
        let Some(relative) = name.strip_prefix(prefix) else {
            continue;
        };
        if relative.is_empty() {
            continue;
        }
        let destination = to.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination).map_err(|source| Error::Write {
                path: destination.display().to_string(),
                source,
            })?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Write {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let mut output = std::fs::File::create(&destination).map_err(|source| Error::Write {
            path: destination.display().to_string(),
            source,
        })?;
        std::io::copy(&mut entry, &mut output).map_err(|source| Error::Write {
            path: destination.display().to_string(),
            source,
        })?;
        extracted += 1;
    }
    log::info!("extracted {extracted} modpack override files");
    Ok(())
}

fn copy_recursively(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursively(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn copies_a_directory_tree() {
        let source =
            std::env::temp_dir().join(format!("conic-overrides-source-{}", uuid::Uuid::new_v4()));
        let destination = std::env::temp_dir().join(format!(
            "conic-overrides-destination-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(source.join("overrides/config")).expect("the pack tree");
        std::fs::write(source.join("overrides/config/a.toml"), "value = true\n")
            .expect("an override");
        extract_all(
            &[Overrides::Directory {
                base: source.clone(),
                relative: "overrides".to_string(),
            }],
            &destination,
        )
        .await
        .expect("the overrides");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&destination).ok();
    }

    #[tokio::test]
    async fn extracts_an_archive_prefix() {
        use std::io::Write as _;

        let archive = std::env::temp_dir().join(format!(
            "conic-overrides-archive-{}.zip",
            uuid::Uuid::new_v4()
        ));
        {
            let file = std::fs::File::create(&archive).expect("a zip file");
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("pack/overrides/config/a.toml", options)
                .expect("the override entry");
            zip.write_all(b"value = true\n").expect("the override");
            zip.finish().expect("a finished zip");
        }
        let destination =
            std::env::temp_dir().join(format!("conic-overrides-unpacked-{}", uuid::Uuid::new_v4()));
        extract_all(
            &[Overrides::Archive {
                path: archive.clone(),
                prefix: "pack/overrides/".to_string(),
            }],
            &destination,
        )
        .await
        .expect("the overrides");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        std::fs::remove_file(&archive).ok();
        std::fs::remove_dir_all(&destination).ok();
    }
}
