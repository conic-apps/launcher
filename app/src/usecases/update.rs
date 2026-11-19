// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The self-update use case: decide whether to look, run the check, and stage the
//! bundle. The swap happens as the process exits; the next launch with the new
//! build posts the matching "updated" notice. Failures stay in the log.

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

/// Maps the config's channel onto the server's two; `nightly` is no longer a
/// server channel and is served the stable one.
pub(crate) fn channel(configured: &config::UpdateChannel) -> Channel {
    match configured {
        config::UpdateChannel::Beta => Channel::Beta,
        _ => Channel::Stable,
    }
}

/// The version the previous session applied, when this build is that version; a
/// record naming another version is dropped.
pub(crate) fn applied_version() -> Option<String> {
    match update::take_applied() {
        Ok(Some(applied)) if applied.matches_current() => Some(applied.version),
        Ok(Some(applied)) => {
            log::warn!(
                "an update to {} was recorded, but this is {}; dropping the record",
                applied.version,
                shared::app_version()
            );
            None
        }
        Ok(None) => None,
        Err(error) => {
            log::warn!("could not read the applied-update record: {error}");
            None
        }
    }
}

/// Runs one check and, when an update is available, the staging download.
///
/// Returns the version that was staged, for the caller to announce.
pub(crate) async fn run(configured_channel: config::UpdateChannel) -> Option<String> {
    let Some(kind) = install_form().kind() else {
        log::info!("update check skipped: this install is managed by a package manager");
        return None;
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
            return None;
        }
        Err(error) => {
            log::warn!("update check failed: {error}");
            return None;
        }
    };

    log::info!("update {} is available; downloading", info.version);
    match update::download_and_stage(&info, kind, shared::silent()).await {
        Ok(staged) => {
            log::info!(
                "update {} downloaded and staged; it is applied as the launcher exits",
                staged.version
            );
            Some(staged.version)
        }
        Err(error) => {
            log::warn!("update download failed: {error}");
            None
        }
    }
}
