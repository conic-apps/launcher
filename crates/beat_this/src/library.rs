// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{ffi::OsStr, path::Path};

use crate::{
    error::Result,
    metadata::{self, LIBRARY},
};
use download::{DownloadTask, progress::DownloadState};
use folder::DATA_LOCATION;
use libloader::libloading::Library;
use sha2::Digest;

pub async fn ensure_library() -> Result<()> {
    let mut sha256_hasher = sha2::Sha256::new();
    let file_content = async_fs::read(
        &DATA_LOCATION
            .runtime
            .join("beat-this")
            .join(LIBRARY.filename),
    )
    .await?;
    sha256_hasher.update(file_content);
    let sha256 = format!("{:02x}", sha256_hasher.finalize());
    let checksum_matched = metadata::LIBRARY.sha256 == sha256;
    if checksum_matched {
        return Ok(());
    }
    let progress = DownloadState::default();
    download_library(&progress).await
}

pub async fn download_library(progress: &DownloadState) -> Result<()> {
    let library_path = DATA_LOCATION
        .runtime
        .join("beat-this")
        .join(LIBRARY.filename);
    for source in LIBRARY.sources {
        let download_task = DownloadTask {
            url: source.to_string(),
            file: library_path.clone(),
            size_bytes: None,
            checksum: download::Checksum::Sha256(LIBRARY.sha256.to_string()),
            task_type: download::DownloadTaskType::BeatThis,
        };
        if download::download(&download_task, progress).await.is_ok() {
            return Ok(());
        };
    }
    Err(crate::error::Error::AllSourceFailed)
}

/// # Safety
///
/// When a library is loaded, initialisation routines contained within it are executed.
/// For the purposes of safety, the execution of these routines is conceptually the same calling an
/// unknown foreign function and may impose arbitrary requirements on the caller for the call
/// to be sound.
pub async unsafe fn load_library_from_file<P: AsRef<OsStr> + AsRef<Path>>(
    path: P,
) -> Result<Library> {
    let mut sha256_hasher = sha2::Sha256::new();
    let file_content = async_fs::read(&path).await?;
    sha256_hasher.update(file_content);
    let sha256 = format!("{:02x}", sha256_hasher.finalize());
    let checksum_matched = metadata::LIBRARY.sha256 == sha256;
    if !checksum_matched {
        return Err(crate::error::Error::ChecksumMismatch);
    }
    unsafe { Ok(Library::new(path)?) }
}
