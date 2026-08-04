// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    error::*,
    metadata::{self, LIBRARY},
};
use download::{DownloadTask, progress::DownloadState};
use folder::DATA_LOCATION;
use sha2::Digest;

pub async fn check_library_valid() -> Result<()> {
    let mut sha256_hasher = sha2::Sha256::new();
    let file_content = async_fs::read(
        &DATA_LOCATION
            .runtime
            .join("conic-nexus")
            .join(LIBRARY.filename),
    )
    .await?;
    sha256_hasher.update(file_content);
    let sha256 = format!("{:02x}", sha256_hasher.finalize());
    (metadata::LIBRARY.sha256 == sha256)
        .then_some(())
        .ok_or(Error::ChecksumMismatch)
}

pub async fn download_library(progress: &DownloadState) -> Result<()> {
    let library_path = DATA_LOCATION
        .runtime
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
        dbg!(&download_task);
        if download::download(&download_task, progress).await.is_ok() {
            return Ok(());
        };
    }
    Err(crate::error::Error::AllSourceFailed)
}
