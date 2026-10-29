// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app-side modpack install.
//!
//! The import itself lives in the `modpack` crate; this runs it with the
//! launcher's config and adapts its two progress vocabularies into the task
//! centre's one.

use std::sync::Arc;

use config::Config;
use shared::{
    Sink,
    task::{TaskProgress, TaskStage},
};

use crate::usecases::tasks::TaskHandle;

pub(crate) async fn install(
    id: String,
    plan: modpack::Plan,
    config: Config,
    task: TaskHandle,
) -> modpack::Result<()> {
    let sink: Sink<modpack::Progress> = {
        let task = task.clone();
        Arc::new(move |progress| match progress {
            modpack::Progress::Install(progress) => {
                let (stage, progress) = crate::usecases::progress::install(&progress);
                task.update(stage, progress);
            }
            modpack::Progress::Download(snapshot) => {
                let (stage, progress) = crate::usecases::progress::download(&snapshot);
                task.update(stage, progress);
            }
            modpack::Progress::Extracting => {
                task.update(TaskStage::Extracting, TaskProgress::Indeterminate);
            }
        })
    };
    modpack::install(id, plan, config, sink).await
}
