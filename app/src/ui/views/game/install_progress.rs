// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The install progress drawn beside the launch button.
//!
//! This is the mapping the launch screen used to own: the install crate reports
//! a stage and, for a download, the byte counts it has and wants. Every stage the
//! old progress panel described is still described here — it is the instance's
//! install that the player now starts from the game view, so the button is where
//! it is reported.

use crate::slint_backend::GameState;
use crate::support::formatting::bytes;
use download::progress::{DownloadPhase, DownloadSnapshot};
use install::{InstallProgress, ModLoaderProgress};

/// One write to the strip, computed off the UI thread.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InstallView {
    kind: &'static str,
    name: String,
    current: String,
    total: String,
    message: String,
    loading: bool,
    value: f32,
    max: f32,
}

impl InstallView {
    fn stage(kind: &'static str, name: &str) -> Self {
        Self {
            kind,
            name: name.to_string(),
            loading: true,
            ..Self::default()
        }
    }

    /// A download: a determinate bar when there is a total to count against, and
    /// the plain stage text otherwise.
    fn download(
        kind: &'static str,
        plain_kind: &'static str,
        name: String,
        snapshot: &DownloadSnapshot,
    ) -> Self {
        if snapshot.phase == DownloadPhase::VerifyExistingFiles || snapshot.total_bytes == 0 {
            return Self::stage(plain_kind, &name);
        }
        Self {
            kind,
            name,
            current: bytes(snapshot.completed_bytes),
            total: bytes(snapshot.total_bytes),
            loading: false,
            value: snapshot.completed_bytes as f32,
            max: snapshot.total_bytes as f32,
            ..Self::default()
        }
    }
}

/// The view for an install event. `loader` is the instance's mod loader name,
/// which the loader stages show.
pub(crate) fn install_view(progress: &InstallProgress, loader: &str) -> InstallView {
    match progress {
        InstallProgress::Prepare => InstallView::stage("prepare-download", ""),
        InstallProgress::InstallGame(snapshot) => {
            InstallView::download("download-files", "verify-files", String::new(), snapshot)
        }
        InstallProgress::InstallJava(snapshot) => {
            InstallView::download("download-java", "check-java", String::new(), snapshot)
        }
        InstallProgress::InstallModLoader(progress) => match progress {
            ModLoaderProgress::Prepare => InstallView::stage("install-mod-loader", loader),
            ModLoaderProgress::DownloadInstaller(snapshot) => InstallView::download(
                "download-installer-progress",
                "download-installer",
                loader.to_string(),
                snapshot,
            ),
            ModLoaderProgress::PrefetchDependencies(snapshot) => InstallView::download(
                "download-deps-progress",
                "download-deps",
                loader.to_string(),
                snapshot,
            ),
            ModLoaderProgress::RunInstaller { message } => InstallView {
                kind: "run-installer",
                name: loader.to_string(),
                message: message.clone(),
                loading: true,
                ..InstallView::default()
            },
        },
    }
}

/// Writes one strip state. The text itself is translated in the UI, from
/// `kind` plus these parts, so a language change re-renders it.
pub(crate) fn apply(state: &GameState, view: &InstallView) {
    state.set_install_kind(view.kind.into());
    state.set_install_name(view.name.as_str().into());
    state.set_install_current(view.current.as_str().into());
    state.set_install_total(view.total.as_str().into());
    state.set_install_message(view.message.as_str().into());
    state.set_install_loading(view.loading);
    state.set_install_value(view.value);
    state.set_install_max(view.max);
}

/// The strip's install fields back to their idle values, for a selection with no
/// install of its own.
pub(crate) fn clear(state: &GameState) {
    apply(state, &InstallView::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(phase: DownloadPhase, completed: u64, total: u64) -> DownloadSnapshot {
        DownloadSnapshot {
            phase,
            completed_bytes: completed,
            total_bytes: total,
        }
    }

    #[test]
    fn verifying_collapses_to_the_plain_stage() {
        let view = install_view(
            &InstallProgress::InstallGame(snapshot(DownloadPhase::VerifyExistingFiles, 3, 3)),
            "",
        );
        assert_eq!(view.kind, "verify-files");
        assert!(view.loading);
    }

    #[test]
    fn a_download_formats_its_byte_counts() {
        let view = install_view(
            &InstallProgress::InstallGame(snapshot(DownloadPhase::DownloadFiles, 1024, 2048)),
            "",
        );
        assert_eq!(view.kind, "download-files");
        assert_eq!(view.current, "1.00 KB");
        assert_eq!(view.total, "2.00 KB");
        assert!(!view.loading);
    }

    #[test]
    fn the_loader_name_is_shown_on_its_stages() {
        let view = install_view(
            &InstallProgress::InstallModLoader(ModLoaderProgress::DownloadInstaller(snapshot(
                DownloadPhase::DownloadFiles,
                10,
                20,
            ))),
            "Forge",
        );
        assert_eq!(view.name, "Forge");
        assert_eq!(view.kind, "download-installer-progress");
    }

    #[test]
    fn the_installer_line_survives() {
        let view = install_view(
            &InstallProgress::InstallModLoader(ModLoaderProgress::RunInstaller {
                message: "Extracting".into(),
            }),
            "NeoForge",
        );
        assert_eq!(view.kind, "run-installer");
        assert_eq!(view.message, "Extracting");
        assert!(view.loading);
    }
}
