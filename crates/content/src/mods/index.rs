// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Per-instance file fingerprints for the mods folder.
//!
//! A SHA-512 is the content identity everything else in [`super::remote`] is
//! keyed by, but calculating it means reading the whole jar. This index records
//! the stat of each file beside the checksum it produced, so an unchanged file
//! is answered from the index instead of being read again. A changed file, or
//! one that is not in the index at all, is hashed once.
//!
//! Paths are the primary key because that is what the directory scan has. An
//! inode is kept beside it so a renamed file can keep its checksum without being
//! read again; on platforms where the standard library does not expose one, the
//! size and modification time still do the common case.
//!
//! The file is per instance because deleting an instance should drop the paths
//! that described it. The checksums themselves are content-addressed and live
//! in `identity.json`, where a jar shared by two instances is stored once.

use std::{
    collections::HashMap,
    fs::Metadata,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::UNIX_EPOCH,
};

use log::warn;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub(crate) type FileIndex = HashMap<String, FileSnapshot>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileSnapshot {
    /// The file's length in bytes.
    len: u64,
    /// The modification time as nanoseconds since the Unix epoch.
    mtime_ns: u64,
    /// The inode where the platform exposes one. It lets a rename reuse the
    /// checksum instead of reading the file again.
    #[serde(default)]
    ino: Option<u64>,
    /// The SHA-512 of the bytes these stat fields described.
    sha512: String,
}

impl FileSnapshot {
    fn matches(&self, len: u64, mtime_ns: u64, ino: Option<u64>) -> bool {
        self.len == len
            && self.mtime_ns == mtime_ns
            && match (self.ino, ino) {
                (Some(stored), Some(current)) => stored == current,
                _ => true,
            }
    }
}

/// One file the scan resolved, in the order the caller should present it.
pub(crate) struct ScannedFile {
    pub(crate) path: PathBuf,
    pub(crate) sha512: String,
}

/// Serializes writes to the per-instance indexes. Reads need no lock: the save
/// is a rename over the old file, so a reader sees either whole version.
static INDEX_WRITE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

async fn index_write_lock() -> tokio::sync::MutexGuard<'static, ()> {
    INDEX_WRITE_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn index_path(instance_id: &str) -> PathBuf {
    storage::LOCATIONS
        .launcher
        .cache
        .join("mods")
        .join("files")
        .join(format!("{instance_id}.json"))
}

pub(crate) async fn load(instance_id: &str) -> FileIndex {
    let path = index_path(instance_id);
    match tokio::fs::read(&path).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(index) => index,
            Err(error) => {
                warn!(
                    "{} could not be read as a mod index ({error}); it will be rebuilt",
                    path.display()
                );
                FileIndex::new()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FileIndex::new(),
        Err(error) => {
            warn!("Could not read {}: {error}", path.display());
            FileIndex::new()
        }
    }
}

pub(crate) async fn save(instance_id: &str, index: &FileIndex) {
    let _guard = index_write_lock().await;
    write_unlocked(instance_id, index).await;
}

async fn write_unlocked(instance_id: &str, index: &FileIndex) {
    let Ok(bytes) = serde_json::to_vec(index) else {
        warn!("Failed to serialize the mod index of {instance_id}");
        return;
    };
    let path = index_path(instance_id);
    let Some(parent) = path.parent() else {
        return;
    };
    if let Err(error) = tokio::fs::create_dir_all(parent).await {
        warn!("Failed to create {}: {error}", parent.display());
        return;
    }
    let temporary = path.with_extension("json.tmp");
    if let Err(error) = tokio::fs::write(&temporary, bytes).await {
        warn!("Failed to write {}: {error}", temporary.display());
        return;
    }
    if let Err(error) = tokio::fs::rename(&temporary, &path).await {
        warn!("Failed to replace {}: {error}", path.display());
    }
}

/// Adds one file to the index immediately after the launcher writes it, so the
/// check that follows the install does not have to hash it a second time.
pub(crate) async fn remember(instance_id: &str, path: &Path, sha512: &str) {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            warn!("Could not stat the installed mod {:?}: {error}", path);
            return;
        }
    };
    if !metadata.is_file() {
        return;
    }
    let Some(name) = path.file_name() else {
        return;
    };
    let name = name.to_string_lossy().into_owned();
    let _guard = index_write_lock().await;
    let mut index = load(instance_id).await;
    index.insert(
        name,
        FileSnapshot {
            len: metadata.len(),
            mtime_ns: mtime_ns(&metadata),
            ino: ino(&metadata),
            sha512: sha512.to_string(),
        },
    );
    write_unlocked(instance_id, &index).await;
}

/// Resolves every file directly inside `folder`, reusing the checksum of an
/// entry that still describes the file on disk.
///
/// The returned index contains only files the scan saw, so entries for files
/// that have gone disappear with them. `previous` may be empty; the first scan
/// of an instance hashes everything.
pub(crate) fn scan(folder: &Path, previous: &FileIndex) -> (FileIndex, Vec<ScannedFile>) {
    let mut by_ino: HashMap<u64, &FileSnapshot> = HashMap::new();
    for snapshot in previous.values() {
        if let Some(ino) = snapshot.ino {
            by_ino.entry(ino).or_insert(snapshot);
        }
    }

    let entries = match folder.read_dir() {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (FileIndex::new(), Vec::new());
        }
        Err(error) => {
            warn!("Could not read {:?}: {error}", folder);
            return (FileIndex::new(), Vec::new());
        }
    };

    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    // `read_dir` has no defined order; a stable one keeps the installed version
    // the detail panel shows from changing between opens.
    paths.sort();

    let mut next = FileIndex::new();
    let mut files = Vec::new();
    for path in paths {
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                warn!("Could not stat {:?}: {error}", path);
                continue;
            }
        };
        if !metadata.is_file() {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        let name = name.to_string_lossy().into_owned();
        let len = metadata.len();
        let mtime_ns = mtime_ns(&metadata);
        let ino = ino(&metadata);

        let reused = previous
            .get(&name)
            .filter(|snapshot| snapshot.matches(len, mtime_ns, ino))
            .map(|snapshot| snapshot.sha512.clone())
            .or_else(|| {
                let ino = ino?;
                by_ino
                    .get(&ino)
                    .filter(|snapshot| snapshot.matches(len, mtime_ns, Some(ino)))
                    .map(|snapshot| snapshot.sha512.clone())
            });
        let sha512 = match reused {
            Some(sha512) => sha512,
            None => match sha512_file(&path) {
                Ok(sha512) => sha512,
                Err(error) => {
                    warn!("Failed to hash {:?}: {error}", path);
                    continue;
                }
            },
        };

        next.insert(
            name,
            FileSnapshot {
                len,
                mtime_ns,
                ino,
                sha512: sha512.clone(),
            },
        );
        files.push(ScannedFile { path, sha512 });
    }
    (next, files)
}

fn mtime_ns(metadata: &Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

fn ino(metadata: &Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
        Some(metadata.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

pub(crate) fn sha512_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, time::SystemTime};

    fn test_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!(
            "conic-mod-index-{name}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create the test folder");
        dir
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        let mut file = std::fs::File::create(path).expect("create the test file");
        file.write_all(bytes).expect("write the test file");
    }

    #[test]
    fn unchanged_files_keep_their_checksum() {
        let dir = test_dir("unchanged");
        let path = dir.join("mod.jar");
        write_file(&path, b"not really a jar");
        let (index, first) = scan(&dir, &FileIndex::new());
        assert_eq!(first.len(), 1);

        let (_, second) = scan(&dir, &index);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].sha512, first[0].sha512);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_changed_file_is_hashed_again() {
        let dir = test_dir("changed");
        let path = dir.join("mod.jar");
        write_file(&path, b"first");
        let (index, first) = scan(&dir, &FileIndex::new());
        write_file(&path, b"second");
        let (_, second) = scan(&dir, &index);
        assert_ne!(second[0].sha512, first[0].sha512);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_valid_snapshot_answers_without_reading_the_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = test_dir("unreadable");
        let path = dir.join("mod.jar");
        write_file(&path, b"not really a jar");
        let (index, first) = scan(&dir, &FileIndex::new());

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
            .expect("make the test file unreadable");
        let (_, second) = scan(&dir, &index);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].sha512, first[0].sha512);

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_rename_reuses_the_checksum_by_inode() {
        let dir = test_dir("rename");
        let path = dir.join("old.jar");
        write_file(&path, b"same bytes");
        let metadata = std::fs::metadata(&path).expect("stat the test file");
        let mut index = FileIndex::new();
        index.insert(
            "old.jar".into(),
            FileSnapshot {
                len: metadata.len(),
                mtime_ns: mtime_ns(&metadata),
                ino: ino(&metadata),
                sha512: "stored-checksum".into(),
            },
        );

        std::fs::rename(&path, dir.join("new.jar")).expect("rename the test file");
        let (_, scanned) = scan(&dir, &index);
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].sha512, "stored-checksum");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn snapshot_validation_requires_the_same_stat() {
        let snapshot = FileSnapshot {
            len: 3,
            mtime_ns: 10,
            ino: Some(7),
            sha512: "h".into(),
        };
        assert!(snapshot.matches(3, 10, Some(7)));
        assert!(!snapshot.matches(4, 10, Some(7)));
        assert!(!snapshot.matches(3, 11, Some(7)));
        assert!(!snapshot.matches(3, 10, Some(8)));
    }
}
