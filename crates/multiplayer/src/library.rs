// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    error::*,
    metadata::{self, LIBRARY},
};
use download::{
    DownloadTask,
    progress::{DownloadSnapshot, DownloadState},
};
use libloader::libloading::Library;
use sha2::Digest;
use shared::ChangeReporter;
use std::{ffi::OsStr, path::Path};
use storage::LOCATIONS;

/// The output port [`download_library`] reports its progress through.
pub type LibrarySink = shared::Sink<DownloadSnapshot>;

/// The directory the Conic Nexus dynamic library lives in
/// (`LOCATIONS.launcher.native/conic-nexus`).
pub fn library_dir() -> std::path::PathBuf {
    LOCATIONS.launcher.native.join("conic-nexus")
}

pub async fn check_library_valid() -> Result<()> {
    let mut sha256_hasher = sha2::Sha256::new();
    let path = LOCATIONS
        .launcher
        .native
        .join("conic-nexus")
        .join(LIBRARY.filename);
    let file_content = tokio::fs::read(&path).await?;
    sha256_hasher.update(file_content);
    let sha256 = format!("{:02x}", sha256_hasher.finalize());
    // The integrity check of a downloaded native library, and it never said
    // anything — the caller only learned "not valid", with no expected-vs-actual
    // and no path, so a tampered or truncated dylib was indistinguishable from a
    // missing one.
    if metadata::LIBRARY.sha256 != sha256 {
        log::error!(
            "The Conic Nexus library at {} has sha256 {sha256}, but {} was expected",
            path.display(),
            metadata::LIBRARY.sha256
        );
        return Err(Error::ChecksumMismatch);
    }
    Ok(())
}

pub async fn download_library(sink: LibrarySink) -> Result<()> {
    let reporter = ChangeReporter::new(sink);
    let library_path = LOCATIONS
        .launcher
        .native
        .join("conic-nexus")
        .join(LIBRARY.filename);
    for source in LIBRARY.sources {
        let download_task = DownloadTask {
            url: source.to_string(),
            file: library_path.clone(),
            size_bytes: Some(LIBRARY.size),
            checksum: download::Checksum::Sha256(LIBRARY.sha256.to_string()),
            task_type: download::DownloadTaskType::ConicNexus,
        };
        // Each source gets its own counters, so a failed source does not leave
        // its progress behind for the next one.
        let progress = DownloadState::default();
        let result = download::progress::watch(
            &progress,
            |snapshot| reporter.report(snapshot),
            download::download(&download_task, &progress),
        )
        .await;
        match result {
            Ok(()) => return Ok(()),
            // Each source's own reason used to be dropped, so "the library never
            // downloads" was unattributable: only the aggregate `AllSourceFailed`
            // reached the app, naming no URL.
            Err(error) => log::warn!("The Conic Nexus library source {} failed: {error}", source),
        };
    }
    Err(crate::error::Error::AllSourceFailed)
}

/// # Safety
///
/// When a library is loaded, initialisation routines contained within it are
/// executed. For the purposes of safety, the execution of these routines is
/// conceptually the same calling an unknown foreign function and may impose
/// arbitrary requirements on the caller for the call to be sound.
pub async unsafe fn load_library_from_file<P: AsRef<OsStr> + AsRef<Path>>(
    path: P,
) -> Result<Library> {
    let mut sha256_hasher = sha2::Sha256::new();
    let file_content = tokio::fs::read(&path).await?;
    sha256_hasher.update(file_content);
    let sha256 = format!("{:02x}", sha256_hasher.finalize());
    let checksum_matched = metadata::LIBRARY.sha256 == sha256;
    if !checksum_matched {
        return Err(crate::error::Error::ChecksumMismatch);
    }
    unsafe { Ok(Library::new(path)?) }
}
