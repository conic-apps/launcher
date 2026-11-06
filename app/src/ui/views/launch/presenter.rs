// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch screen's presenter: the launch crate's progress → a [`LaunchView`]
//! → `LaunchState`. The mapping is pure and unit-tested without a window;
//! [`apply_view`] is the only Slint-touching line.
//!
//! Install progress is not shown here any more: it belongs to the game view's
//! button (`app/src/ui/views/game/install_progress.rs`).

use crate::slint_backend::LaunchState;
use download::progress::DownloadPhase;
use launch::LaunchProgress;

/// One write to the progress screen, computed off the UI thread.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct LaunchView {
    kind: &'static str,
    current: String,
    total: String,
    loading: bool,
    value: f32,
    max: f32,
    back_disabled: bool,
}

impl LaunchView {
    fn stage(kind: &'static str) -> Self {
        Self {
            kind,
            loading: true,
            ..Self::default()
        }
    }
}

/// The view for a launch event, or `None` for the ones the screen does not show.
pub(crate) fn launch_view(progress: &LaunchProgress) -> Option<LaunchView> {
    let view = match progress {
        LaunchProgress::Prepare => LaunchView::stage("preparing-launch"),
        // The authlib-injector step updates no screen state.
        LaunchProgress::InstallAuthlibInjector(_) => return None,
        LaunchProgress::CompleteFiles(snapshot) => {
            if snapshot.phase == DownloadPhase::VerifyExistingFiles {
                LaunchView::stage("verify-files")
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
        LaunchProgress::GenerateScriptlet => LaunchView::stage("generate-script"),
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

pub(crate) fn apply_view(state: &LaunchState, view: LaunchView) {
    state.set_progress_kind(view.kind.into());
    state.set_progress_current(view.current.into());
    state.set_progress_total(view.total.into());
    state.set_progress_loading(view.loading);
    state.set_progress_value(view.value);
    state.set_progress_max(view.max);
    state.set_back_disabled(view.back_disabled);
}

#[cfg(test)]
mod tests {
    use super::*;
    use download::progress::DownloadSnapshot;

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
    fn launch_bytes_stay_raw() {
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
