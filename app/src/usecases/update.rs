// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The self-update use case: decide whether to look, run the check, and stage
//! the bundle.
//!
//! Deliberately silent. It runs once at startup, reports every step to the log
//! and touches no interface; the staged bundle is swapped in as the process
//! exits by `update::apply_pending`. That keeps the update out of the user's
//! way — the next launch is simply the new version.

use std::sync::OnceLock;

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

/// One check, followed by a staging download if something newer exists.
pub(crate) async fn run(configured_channel: config::UpdateChannel) {
    let Some(kind) = install_form().kind() else {
        log::info!("update check skipped: this install is managed by a package manager");
        return;
    };

    log::info!(
        "checking for updates on the {} channel (running {})",
        channel(&configured_channel).as_str(),
        shared::APP_VERSION
    );
    let info = match update::check(channel(&configured_channel), kind, shared::APP_VERSION).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            log::info!("no update available (running {})", shared::APP_VERSION);
            return;
        }
        Err(error) => {
            log::warn!("update check failed: {error}");
            return;
        }
    };

    log::info!("update {} is available; downloading", info.version);
    match update::download_and_stage(&info, kind, shared::silent()).await {
        Ok(staged) => log::info!(
            "update {} downloaded and staged; it will be applied as the launcher exits",
            staged.version
        ),
        Err(error) => log::warn!("update download failed: {error}"),
    }
}
