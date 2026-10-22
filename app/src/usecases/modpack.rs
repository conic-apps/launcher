// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app-side modpack install.
//!
//! The import itself lives in the `modpack` crate; this runs it with the
//! launcher's config. TODO: show the progress in the UI — until then the
//! crate's progress goes to the log.

use std::sync::Arc;

use config::Config;
use shared::Sink;

pub(crate) async fn install(
    id: String,
    plan: modpack::Plan,
    config: Config,
) -> modpack::Result<()> {
    // TODO: surface these on an install screen instead of the log.
    let sink: Sink<modpack::Progress> = Arc::new(|progress| match progress {
        modpack::Progress::Install(progress) => log::info!("modpack install: {progress:?}"),
        modpack::Progress::Download(snapshot) => log::info!("modpack download: {snapshot:?}"),
    });
    modpack::install(id, plan, config, sink).await
}
