// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The modpack install's progress, mapped onto the launch screen's port.
//!
//! The import itself lives in the `modpack` crate; this is the one place that
//! knows both the crate's `Progress` and the launch use case's `LaunchUpdate`.
//! The finished/error update is also the launch screen's: an import ends by
//! returning to the instance list, not by launching a game.

use std::sync::Arc;

use config::Config;

use super::launch::{LaunchSink, LaunchUpdate};

pub(crate) async fn install(id: String, plan: modpack::Plan, config: Config, out: LaunchSink) {
    let progress_out = out.clone();
    let sink: modpack::ProgressSink = Arc::new(move |progress| match progress {
        modpack::Progress::Install(progress) => progress_out(LaunchUpdate::Install(progress)),
        modpack::Progress::Download(snapshot) => progress_out(LaunchUpdate::Modpack(snapshot)),
    });
    match modpack::install(id, plan, config, sink).await {
        Ok(()) => out(LaunchUpdate::Finished {
            quit_app: false,
            pause_music: false,
        }),
        Err(error) => out(LaunchUpdate::Error(error.to_string())),
    }
}
