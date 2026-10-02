// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch screen's presenter: a crate's progress → a [`LaunchView`] →
//! `LaunchState`.
//!
//! This is the testable half of the Humble Object pair. It decides how a byte
//! count becomes text and which stage shows a spinner, but it does not know
//! Slint: the mapping is a pure function of a crate event, so it is unit-tested
//! without a window, and [`apply`] is the only Slint-touching line.

use crate::slint_backend::LaunchState;
use crate::support::formatting::bytes;
use download::progress::{DownloadPhase, DownloadSnapshot};
use install::{InstallProgress, ModLoaderProgress};
use launch::LaunchProgress;

/// One write to the progress screen, computed off the UI thread.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct LaunchView {
    kind: &'static str,
    name: String,
    current: String,
    total: String,
    message: String,
    loading: bool,
    value: f32,
    max: f32,
    back_disabled: bool,
}

impl LaunchView {
    /// A stage that shows only its text and the spinner.
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
pub(crate) fn install_view(progress: &InstallProgress, loader: &str) -> LaunchView {
    match progress {
        InstallProgress::Prepare => LaunchView::stage("prepare-download", ""),
        InstallProgress::InstallGame(snapshot) => {
            LaunchView::download("download-files", "verify-files", String::new(), snapshot)
        }
        InstallProgress::InstallJava(snapshot) => {
            LaunchView::download("download-java", "check-java", String::new(), snapshot)
        }
        InstallProgress::InstallModLoader(progress) => match progress {
            ModLoaderProgress::Prepare => LaunchView::stage("install-mod-loader", loader),
            ModLoaderProgress::DownloadInstaller(snapshot) => LaunchView::download(
                "download-installer-progress",
                "download-installer",
                loader.to_string(),
                snapshot,
            ),
            ModLoaderProgress::PrefetchDependencies(snapshot) => LaunchView::download(
                "download-deps-progress",
                "download-deps",
                loader.to_string(),
                snapshot,
            ),
            ModLoaderProgress::RunInstaller { message } => LaunchView {
                kind: "run-installer",
                name: loader.to_string(),
                message: message.clone(),
                loading: true,
                ..LaunchView::default()
            },
        },
    }
}

/// The view for a launch event, or `None` for the ones the screen does not show.
pub(crate) fn launch_view(progress: &LaunchProgress) -> Option<LaunchView> {
    let view = match progress {
        LaunchProgress::Prepare => LaunchView::stage("preparing-launch", ""),
        // The authlib-injector step updates no screen state.
        LaunchProgress::InstallAuthlibInjector(_) => return None,
        LaunchProgress::CompleteFiles(snapshot) => {
            if snapshot.phase == DownloadPhase::VerifyExistingFiles {
                LaunchView::stage("verify-files", "")
            } else {
                // Raw byte counts here, unlike the install path, which formats
                // them with `bytes`.
                LaunchView {
                    kind: "download-files",
                    current: snapshot.completed_bytes.to_string(),
                    total: snapshot.total_bytes.to_string(),
                    loading: false,
                    value: snapshot.completed_bytes as f32,
                    max: snapshot.total_bytes as f32,
                    ..LaunchView::default()
                }
            }
        }
        LaunchProgress::GenerateScriptlet => LaunchView::stage("generate-script", ""),
        // The three startup markers share one screen state, but each still
        // disables the back button: the flow is deliberately still running.
        LaunchProgress::WaitForLaunch
        | LaunchProgress::LogSettingUser
        | LaunchProgress::LogLwjglVersion
        | LaunchProgress::LogOpenALLoaded => LaunchView {
            kind: "wait-for-launch",
            loading: true,
            back_disabled: true,
            ..LaunchView::default()
        },
        LaunchProgress::LogTextureLoaded => LaunchView {
            kind: "game-started",
            loading: true,
            back_disabled: true,
            ..LaunchView::default()
        },
    };
    Some(view)
}

/// Writes one view into the launch screen's global.
pub(crate) fn apply(state: &LaunchState, view: LaunchView) {
    state.set_progress_kind(view.kind.into());
    state.set_progress_name(view.name.into());
    state.set_progress_current(view.current.into());
    state.set_progress_total(view.total.into());
    state.set_progress_message(view.message.into());
    state.set_progress_loading(view.loading);
    state.set_progress_value(view.value);
    state.set_progress_max(view.max);
    state.set_back_disabled(view.back_disabled);
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
        let view = launch_view(&LaunchProgress::CompleteFiles(snapshot(
            DownloadPhase::VerifyExistingFiles,
            3,
            3,
        )))
        .expect("shown");
        assert_eq!(view.kind, "verify-files");
        assert!(view.loading);
    }

    #[test]
    fn install_bytes_are_formatted_and_launch_bytes_are_raw() {
        let install = install_view(
            &InstallProgress::InstallGame(snapshot(DownloadPhase::DownloadFiles, 1024, 2048)),
            "",
        );
        assert_eq!(install.current, "1.00 KB");
        assert_eq!(install.total, "2.00 KB");

        let launch = launch_view(&LaunchProgress::CompleteFiles(snapshot(
            DownloadPhase::DownloadFiles,
            1024,
            2048,
        )))
        .expect("shown");
        assert_eq!(launch.current, "1024");
        assert_eq!(launch.total, "2048");
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
    fn authlib_injector_is_not_shown() {
        assert!(
            launch_view(&LaunchProgress::InstallAuthlibInjector(snapshot(
                DownloadPhase::DownloadFiles,
                0,
                0,
            )))
            .is_none()
        );
    }
}
