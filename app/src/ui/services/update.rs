// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The self-update notices: the bundle waiting for a restart to be installed,
//! and the update the previous session applied.
//!
//! The use case owns the check, the staging and the one-shot record; this module
//! turns their results into cards and wires each card's action.

use shared::notification::{Notice, NoticeKind, NoticeSink};

use crate::support::runtime;
use crate::ui::services::app_config;
use crate::usecases::update;

/// Starts the background check and staging download. The staged bundle is swapped
/// in as the process exits, so the card's action restarts the launcher.
pub(crate) fn spawn_check(channel: config::UpdateChannel, notices: NoticeSink) {
    runtime::spawn(async move {
        let Some(version) = update::run(channel).await else {
            return;
        };
        notices(
            Notice::new(NoticeKind::UpdateReady)
                .with_arg(version)
                .on_click(app_config::relaunch),
        );
    });
}

/// Posts the version the previous session applied; the card opens that version's
/// GitHub release, whose notes are the changelog.
pub(crate) fn notify_applied(notices: NoticeSink) {
    let Some(version) = update::applied_version() else {
        return;
    };
    log::info!("the launcher was updated to {version}");
    let url = release_url(&version);
    notices(
        Notice::new(NoticeKind::UpdateApplied)
            .with_arg(version)
            .on_click(move || {
                if let Err(error) = app_config::open_external(&url) {
                    log::warn!("failed to open {url}: {error}");
                }
            }),
    );
}

fn release_url(version: &str) -> String {
    let version = version.trim().trim_start_matches(['v', 'V']);
    format!("https://github.com/conic-apps/launcher/releases/tag/v{version}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_url_uses_the_v_prefixed_tag() {
        assert_eq!(
            release_url("0.1.0-alpha.7"),
            "https://github.com/conic-apps/launcher/releases/tag/v0.1.0-alpha.7"
        );
        assert_eq!(
            release_url("v0.1.0-alpha.7"),
            "https://github.com/conic-apps/launcher/releases/tag/v0.1.0-alpha.7"
        );
    }
}
