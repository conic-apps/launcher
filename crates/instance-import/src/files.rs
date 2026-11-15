// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The file trees an import writes into the instance root, and the copy
//! progress they report.
//!
//! A modpack hands over its `overrides` tree; another launcher hands over the
//! game directory it has on disk. Both are one [`FileTree`]: a folder or an
//! archive prefix, plus what of it is the source's own metadata.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Error, Result};

/// How much of the trees has been written, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CopySnapshot {
    pub(crate) done: u64,
    pub(crate) total: u64,
}

pub(crate) type CopySink = Arc<dyn Fn(CopySnapshot) + Send + Sync>;

/// How much copying passes between two reports. Small enough for a live bar,
/// large enough that a tree of thousands of small files is not one event each.
const REPORT_EVERY: u64 = 1 << 20;

/// One tree of source files and what to leave out of it.
#[derive(Clone)]
pub(crate) struct FileTree {
    source: TreeSource,
    skip: Vec<String>,
}

#[derive(Clone)]
enum TreeSource {
    Directory {
        base: PathBuf,
        /// The tree's folder inside `base`; empty when `base` is the tree.
        relative: String,
    },
    Archive {
        path: PathBuf,
        /// The entry prefix to strip before writing; e.g. `pack/overrides/`.
        prefix: String,
    },
}

impl FileTree {
    pub(crate) fn directory(base: PathBuf, relative: String) -> Self {
        Self {
            source: TreeSource::Directory { base, relative },
            skip: Vec::new(),
        }
    }

    pub(crate) fn archive(path: PathBuf, prefix: String) -> Self {
        Self {
            source: TreeSource::Archive { path, prefix },
            skip: Vec::new(),
        }
    }

    /// Leaves `paths` out of the copy.
    ///
    /// A source's own manifests sit inside the tree it hands over (HMCL keeps
    /// its `pack.json` at the top of `minecraft/`), and they must not become
    /// game files. Naming a folder leaves its whole subtree out.
    pub(crate) fn skipping(mut self, paths: &[&str]) -> Self {
        self.skip.extend(paths.iter().map(|path| path.to_string()));
        self
    }

    pub(crate) fn skipping_owned(mut self, paths: Vec<String>) -> Self {
        self.skip.extend(paths);
        self
    }

    pub(crate) fn is_archive(&self) -> bool {
        matches!(self.source, TreeSource::Archive { .. })
    }

    fn bytes(&self) -> Result<u64> {
        match &self.source {
            TreeSource::Directory { base, relative } => {
                directory_bytes(&tree_root(base, relative), "", &self.skip)
            }
            TreeSource::Archive { path, prefix } => archive_bytes(path, prefix, &self.skip),
        }
    }

    fn write(&self, root: &Path, progress: &mut CopyProgress<'_>) -> Result<()> {
        match &self.source {
            TreeSource::Directory { base, relative } => {
                let from = tree_root(base, relative);
                if !from.is_dir() {
                    log::warn!("the import has no {} directory", from.display());
                    return Ok(());
                }
                copy_recursively(&from, root, "", &self.skip, progress)
            }
            TreeSource::Archive { path, prefix } => {
                extract_archive(path, prefix, root, &self.skip, progress)
            }
        }
    }
}

fn tree_root(base: &Path, relative: &str) -> PathBuf {
    if relative.is_empty() {
        base.to_path_buf()
    } else {
        base.join(relative)
    }
}

/// Writes every tree over `instance_root`, in order, reporting the bytes as
/// they are copied.
pub(crate) async fn write_all(
    trees: Vec<FileTree>,
    instance_root: PathBuf,
    on_copy: CopySink,
) -> Result<()> {
    if trees.is_empty() {
        return Ok(());
    }
    tokio::task::spawn_blocking(move || {
        let total = trees
            .iter()
            .try_fold(0u64, |sum, tree| Ok::<_, Error>(sum + tree.bytes()?))?;
        let mut progress = CopyProgress::new(&on_copy, total);
        for tree in &trees {
            tree.write(&instance_root, &mut progress)?;
        }
        progress.finish();
        Ok(())
    })
    .await
    .map_err(|error| Error::Task(error.to_string()))?
}

struct CopyProgress<'a> {
    sink: &'a CopySink,
    total: u64,
    done: u64,
    reported: u64,
}

impl<'a> CopyProgress<'a> {
    fn new(sink: &'a CopySink, total: u64) -> Self {
        Self {
            sink,
            total,
            done: 0,
            reported: 0,
        }
    }

    fn add(&mut self, bytes: u64) {
        self.done += bytes;
        if self.done - self.reported >= REPORT_EVERY {
            self.report();
        }
    }

    fn report(&mut self) {
        self.reported = self.done;
        (self.sink)(CopySnapshot {
            done: self.done,
            total: self.total,
        });
    }

    fn finish(&mut self) {
        if self.total > 0 && self.done != self.reported {
            self.report();
        }
    }
}

fn directory_bytes(from: &Path, relative: &str, skip: &[String]) -> Result<u64> {
    if !from.is_dir() {
        return Ok(0);
    }
    let mut bytes = 0u64;
    let entries = std::fs::read_dir(from).map_err(|source| Error::Read {
        path: from.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: from.display().to_string(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        let child = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        if skipped(&child, skip) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            bytes += directory_bytes(&path, &child, skip)?;
        } else if let Ok(metadata) = entry.metadata() {
            bytes += metadata.len();
        }
    }
    Ok(bytes)
}

fn archive_bytes(path: &Path, prefix: &str, skip: &[String]) -> Result<u64> {
    let mut archive = open_archive(path)?;
    let mut bytes = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|source| Error::Zip {
            path: path.display().to_string(),
            source,
        })?;
        let Some(relative) = entry_name(&entry, path, prefix) else {
            continue;
        };
        if skipped(&relative, skip) {
            continue;
        }
        bytes += entry.size();
    }
    Ok(bytes)
}

fn extract_archive(
    path: &Path,
    prefix: &str,
    to: &Path,
    skip: &[String],
    progress: &mut CopyProgress<'_>,
) -> Result<()> {
    let mut archive = open_archive(path)?;
    let mut extracted = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|source| Error::Zip {
            path: path.display().to_string(),
            source,
        })?;
        let Some(relative) = entry_name(&entry, path, prefix) else {
            continue;
        };
        if skipped(&relative, skip) {
            continue;
        }
        let destination = to.join(&relative);
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
        let written = std::io::copy(&mut entry, &mut output).map_err(|source| Error::Write {
            path: destination.display().to_string(),
            source,
        })?;
        progress.add(written);
        extracted += 1;
    }
    log::info!("wrote {extracted} files out of {}", path.display());
    Ok(())
}

fn open_archive(path: &Path) -> Result<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    zip::ZipArchive::new(file).map_err(|source| Error::Zip {
        path: path.display().to_string(),
        source,
    })
}

/// The entry's path below `prefix`, or `None` for an entry outside it.
///
/// `enclosed_name` rejects `..` and absolute entries, so a source cannot write
/// outside the instance root.
fn entry_name<R: std::io::Read>(
    entry: &zip::read::ZipFile<'_, R>,
    path: &Path,
    prefix: &str,
) -> Option<String> {
    let Some(name) = entry.enclosed_name() else {
        log::warn!(
            "skipping {} in {}: it is not an enclosed path",
            entry.name(),
            path.display()
        );
        return None;
    };
    let name = name.to_string_lossy().replace('\\', "/");
    let relative = name.strip_prefix(prefix)?;
    (!relative.is_empty()).then(|| relative.to_string())
}

fn copy_recursively(
    from: &Path,
    to: &Path,
    relative: &str,
    skip: &[String],
    progress: &mut CopyProgress<'_>,
) -> Result<()> {
    std::fs::create_dir_all(to).map_err(|source| Error::Write {
        path: to.display().to_string(),
        source,
    })?;
    let entries = std::fs::read_dir(from).map_err(|source| Error::Read {
        path: from.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: from.display().to_string(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        let child = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        if skipped(&child, skip) {
            continue;
        }
        let destination = to.join(&name);
        if entry.path().is_dir() {
            copy_recursively(&entry.path(), &destination, &child, skip, progress)?;
            continue;
        }
        let copied = std::fs::copy(entry.path(), &destination).map_err(|source| Error::Write {
            path: destination.display().to_string(),
            source,
        })?;
        progress.add(copied);
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

    fn noop() -> CopySink {
        Arc::new(|_| {})
    }

    #[tokio::test]
    async fn copies_a_directory_tree() {
        let source =
            std::env::temp_dir().join(format!("conic-files-source-{}", uuid::Uuid::new_v4()));
        let destination =
            std::env::temp_dir().join(format!("conic-files-target-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(source.join("overrides/config")).expect("the source tree");
        std::fs::write(source.join("overrides/config/a.toml"), "value = true\n")
            .expect("a source file");
        write_all(
            vec![FileTree::directory(source.clone(), "overrides".to_string())],
            destination.clone(),
            noop(),
        )
        .await
        .expect("the tree");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&destination).ok();
    }

    #[tokio::test]
    async fn leaves_the_source_metadata_out_of_the_copy() {
        let source =
            std::env::temp_dir().join(format!("conic-files-source-{}", uuid::Uuid::new_v4()));
        let destination =
            std::env::temp_dir().join(format!("conic-files-target-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(source.join("minecraft/versions/1.20.1")).expect("the source tree");
        std::fs::write(source.join("minecraft/pack.json"), "{}").expect("the source manifest");
        std::fs::write(source.join("minecraft/options.txt"), "lang:en_us\n").expect("an option");
        std::fs::write(source.join("minecraft/versions/1.20.1/a.json"), "{}").expect("a version");
        write_all(
            vec![
                FileTree::directory(source.clone(), "minecraft".to_string())
                    .skipping(&["pack.json", "versions"]),
            ],
            destination.clone(),
            noop(),
        )
        .await
        .expect("the tree");
        assert!(destination.join("options.txt").is_file());
        assert!(!destination.join("pack.json").exists());
        assert!(!destination.join("versions").exists());
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&destination).ok();
    }

    #[tokio::test]
    async fn extracts_an_archive_prefix_and_reports_the_bytes() {
        use std::io::Write as _;
        use std::sync::Mutex;

        let archive =
            std::env::temp_dir().join(format!("conic-files-archive-{}.zip", uuid::Uuid::new_v4()));
        {
            let file = std::fs::File::create(&archive).expect("a zip file");
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("pack/overrides/config/a.toml", options)
                .expect("a source entry");
            zip.write_all(b"value = true\n").expect("the source file");
            zip.start_file("pack/overrides/pack.json", options)
                .expect("the metadata entry");
            zip.write_all(b"{}").expect("the metadata");
            zip.finish().expect("a finished zip");
        }
        let destination =
            std::env::temp_dir().join(format!("conic-files-unpacked-{}", uuid::Uuid::new_v4()));
        let reports: Arc<Mutex<Vec<CopySnapshot>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let reports = Arc::clone(&reports);
            Arc::new(move |snapshot| reports.lock().expect("the reports").push(snapshot))
        };
        write_all(
            vec![
                FileTree::archive(archive.clone(), "pack/overrides/".to_string())
                    .skipping(&["pack.json"]),
            ],
            destination.clone(),
            sink,
        )
        .await
        .expect("the tree");
        assert_eq!(
            std::fs::read_to_string(destination.join("config/a.toml")).expect("the copied file"),
            "value = true\n"
        );
        assert!(!destination.join("pack.json").exists());
        let reports = reports.lock().expect("the reports");
        let last = reports.last().expect("a report");
        assert_eq!(last.done, last.total);
        assert_eq!(last.total, "value = true\n".len() as u64);
        drop(reports);
        std::fs::remove_file(&archive).ok();
        std::fs::remove_dir_all(&destination).ok();
    }
}
