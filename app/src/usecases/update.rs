// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The self-update use case: decide whether to look, run the check, and stage
//! the bundle.
//!
//! It runs once at startup and reports every step to the log. Once a bundle is
//! staged it posts one notice; the swap itself happens as the process exits
//! (`update::apply_pending`), so the notice is informational rather than a
//! prompt to restart. Failures stay in the log: a background check that found
//! nothing must not interrupt the user.

use std::sync::OnceLock;

use shared::notification::{Notice, NoticeKind, NoticeSink};
use update::{Channel, InstallForm};

/// The install form, detected once. Reading the marker walks the filesystem, so
/// it is done lazily and kept.
static INSTALL_FORM: OnceLock<InstallForm> = OnceLock::new();

pub(crate) fn install_form() -> InstallForm {
    *INSTALL_FORM.get_or_init(update::detect)
}

/// Whether an install like the current one can update itself.
pub(crate) fn is_self_updating() -> bool {
    install_form().is_self_updating()
}

/// Maps the config's channel onto the server's two channels. `nightly` is not a
/// server channel any more; a config that still carries it is served the stable
/// channel rather than failing the request.
pub(crate) fn channel(configured: &config::UpdateChannel) -> Channel {
    match configured {
        config::UpdateChannel::Beta => Channel::Beta,
        _ => Channel::Stable,
    }
}

pub(crate) async fn run(configured_channel: config::UpdateChannel, notices: NoticeSink) {
    let Some(kind) = install_form().kind() else {
        log::info!("update check skipped: this install is managed by a package manager");
        return;
    };

    log::info!(
        "checking for updates on the {} channel (running {})",
        channel(&configured_channel).as_str(),
        shared::app_version()
    );
    let info = match update::check(channel(&configured_channel), kind, shared::app_version()).await
    {
        Ok(Some(info)) => info,
        Ok(None) => {
            log::info!("no update available (running {})", shared::app_version());
            return;
        }
        Err(error) => {
            log::warn!("update check failed: {error}");
            return;
        }
    };

    log::info!("update {} is available; downloading", info.version);
    match update::download_and_stage(&info, kind, shared::silent()).await {
        Ok(staged) => {
            log::info!(
                "update {} downloaded and staged; it will be applied as the launcher exits",
                staged.version
            );
            notices(Notice::new(NoticeKind::UpdateReady).with_arg(staged.version));
        }
        Err(error) => log::warn!("update download failed: {error}"),
    }
}
