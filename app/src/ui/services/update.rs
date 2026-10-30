// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The notice for the update the previous session applied.
//!
//! The use case owns the one-shot record; this module turns it into a card and
//! wires the card's action to the GitHub release of that version, whose notes
//! are the changelog the user is being pointed at.

use shared::notification::{Notice, NoticeKind, NoticeSink};

use crate::ui::services::app_config;
use crate::usecases::update;

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
