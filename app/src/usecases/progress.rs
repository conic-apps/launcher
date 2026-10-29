// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The one place domain progress becomes a task stage.
//!
//! A download reports bytes and a phase; an install reports the same bytes under
//! a named step. Both vocabularies are flattened here so the task centre, the
//! launch page and the modpack import do not each grow their own mapping.

use download::progress::{DownloadPhase, DownloadSnapshot};
use install::{InstallProgress, ModLoaderProgress};
use shared::task::{TaskProgress, TaskStage};

pub(crate) fn download(snapshot: &DownloadSnapshot) -> (TaskStage, TaskProgress) {
    let stage = match snapshot.phase {
        DownloadPhase::VerifyExistingFiles => TaskStage::Verifying,
        DownloadPhase::DownloadFiles => TaskStage::Downloading,
    };
    (stage, bytes(snapshot))
}

pub(crate) fn install(progress: &InstallProgress) -> (TaskStage, TaskProgress) {
    match progress {
        InstallProgress::Prepare => (TaskStage::Preparing, TaskProgress::Indeterminate),
        InstallProgress::InstallGame(snapshot) => (TaskStage::InstallingGame, bytes(snapshot)),
        InstallProgress::InstallJava(snapshot) => (TaskStage::InstallingJava, bytes(snapshot)),
        InstallProgress::InstallModLoader(progress) => match progress {
            ModLoaderProgress::Prepare | ModLoaderProgress::RunInstaller { .. } => {
                (TaskStage::InstallingLoader, TaskProgress::Indeterminate)
            }
            ModLoaderProgress::DownloadInstaller(snapshot)
            | ModLoaderProgress::PrefetchDependencies(snapshot) => {
                (TaskStage::InstallingLoader, bytes(snapshot))
            }
        },
    }
}

fn bytes(snapshot: &DownloadSnapshot) -> TaskProgress {
    if snapshot.total_bytes == 0 {
        TaskProgress::Indeterminate
    } else {
        TaskProgress::Bytes {
            completed: snapshot.completed_bytes,
            total: snapshot.total_bytes,
        }
    }
}
