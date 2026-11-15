// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The pack file trees copied over the instance root after the game is in
//! place, so the pack's configs and `options.txt` win.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// One tree of pack files, and what of it is the pack's own metadata rather
/// than a game file.
#[derive(Clone)]
pub(crate) struct Overrides {
    source: Source,
    skip: Vec<String>,
}

#[derive(Clone)]
enum Source {
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

impl Overrides {
    pub(crate) fn directory(base: PathBuf, relative: String) -> Self {
        Self {
            source: Source::Directory { base, relative },
            skip: Vec::new(),
        }
    }

    pub(crate) fn archive(path: PathBuf, prefix: String) -> Self {
        Self {
            source: Source::Archive { path, prefix },
            skip: Vec::new(),
        }
    }

    /// Leaves `paths` out of the copy.
    ///
    /// A format's own manifests sit inside the tree it hands over (HMCL keeps
    /// its `pack.json` at the top of `minecraft/`), and they must not become
    /// game files. Naming a folder leaves its whole subtree out.
    pub(crate) fn skipping(mut self, paths: &[&str]) -> Self {
        self.skip.extend(paths.iter().map(|path| path.to_string()));
        self
    }

    async fn extract(&self, instance_root: &Path) -> Result<()> {
        match &self.source {
            Source::Directory { base, relative } => {
                if relative.trim().is_empty() {
                    return Ok(());
                }
                let from = base.join(relative);
                if !from.is_dir() {
                    log::warn!("the modpack has no {} directory", from.display());
                    return Ok(());
                }
                let to = instance_root.to_path_buf();
                let skip = self.skip.clone();
                tokio::task::spawn_blocking(move || copy_recursively(&from, &to, "", &skip))
                    .await
                    .map_err(|error| Error::Task(error.to_string()))?
                    .map_err(Error::Overrides)
            }
            Source::Archive { path, prefix } => {
                if prefix.is_empty() {
                    return Ok(());
                }
                let path = path.clone();
                let prefix = prefix.clone();
                let skip = self.skip.clone();
                let to = instance_root.to_path_buf();
                tokio::task::spawn_blocking(move || extract_archive(&path, &prefix, &to, &skip))
                    .await
                    .map_err(|error| Error::Task(error.to_string()))?
            }
        }
    }
}

/// Copies every tree over `instance_root`, in order.
pub(crate) async fn extract_all(overrides: &[Overrides], instance_root: &Path) -> Result<()> {
    for source in overrides {
        source.extract(instance_root).await?;
    }
    Ok(())
}

fn extract_archive(path: &Path, prefix: &str, to: &Path, skip: &[String]) -> Result<()> {
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
        if relative.is_empty() || skipped(relative, skip) {
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

fn copy_recursively(
    from: &Path,
    to: &Path,
    relative: &str,
    skip: &[String],
) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let child = if relative.is_empty() {
                name.clone()
            } else {
                format!("{relative}/{name}")
            };
            if skipped(&child, skip) {
                continue;
            }
            copy_recursively(&entry.path(), &to.join(&name), &child, skip)?;
        }
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
    }
    Ok(())
}

fn skipped(relative: &str, skip: &[String]) -> bool {
    skip.iter().any(|skip| {
        relative == skip
            || relative
                .strip_prefix(skip.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    })
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
            &[Overrides::directory(
                source.clone(),
                "overrides".to_string(),
            )],
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
    async fn leaves_the_pack_metadata_out_of_the_copy() {
        let source =
            std::env::temp_dir().join(format!("conic-overrides-source-{}", uuid::Uuid::new_v4()));
        let destination = std::env::temp_dir().join(format!(
            "conic-overrides-destination-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(source.join("minecraft/versions/1.20.1")).expect("the pack tree");
        std::fs::write(source.join("minecraft/pack.json"), "{}").expect("the pack manifest");
        std::fs::write(source.join("minecraft/options.txt"), "lang:en_us\n").expect("an option");
        std::fs::write(source.join("minecraft/versions/1.20.1/a.json"), "{}").expect("a version");
        extract_all(
            &[
                Overrides::directory(source.clone(), "minecraft".to_string())
                    .skipping(&["pack.json", "versions"]),
            ],
            &destination,
        )
        .await
        .expect("the overrides");
        assert!(destination.join("options.txt").is_file());
        assert!(!destination.join("pack.json").exists());
        assert!(!destination.join("versions").exists());
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
            zip.start_file("pack/overrides/pack.json", options)
                .expect("the metadata entry");
            zip.write_all(b"{}").expect("the metadata");
            zip.finish().expect("a finished zip");
        }
        let destination =
            std::env::temp_dir().join(format!("conic-overrides-unpacked-{}", uuid::Uuid::new_v4()));
        extract_all(
            &[
                Overrides::archive(archive.clone(), "pack/overrides/".to_string())
                    .skipping(&["pack.json"]),
            ],
            &destination,
        )
        .await
        .expect("the overrides");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        assert!(!destination.join("pack.json").exists());
        std::fs::remove_file(&archive).ok();
        std::fs::remove_dir_all(&destination).ok();
    }
}
