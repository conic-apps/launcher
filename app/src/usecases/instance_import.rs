// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app-side import: runs the `instance-import` crate with the launcher's
//! config and adapts its progress vocabulary into the task centre's.
//!
//! The crate reports the new instance id through its progress sink; this is
//! where that becomes the card's target and the frontend's "select this
//! instance" request, so no surface has to drive "create, then install".

use std::sync::Arc;

use config::Config;
use shared::{
    Sink,
    task::{TaskProgress, TaskStage, TaskTarget},
};

use crate::usecases::tasks::TaskHandle;

pub(crate) async fn import(
    plan: instance_import::Plan,
    name: &str,
    config: Config,
    task: TaskHandle,
    on_created: Sink<String>,
) -> instance_import::Result<()> {
    let sink: Sink<instance_import::Progress> = {
        let task = task.clone();
        Arc::new(move |progress| match progress {
            instance_import::Progress::InstanceCreated(id) => {
                task.retarget(TaskTarget::Instance {
                    instance_id: id.clone(),
                });
                on_created(id);
            }
            instance_import::Progress::Game(progress) => {
                let (stage, progress) = crate::usecases::progress::install(&progress);
                task.update(stage, progress);
            }
            instance_import::Progress::Download(snapshot) => {
                let (stage, progress) = crate::usecases::progress::download(&snapshot);
                task.update(stage, progress);
            }
            instance_import::Progress::Copy { done, total } => {
                task.update(
                    TaskStage::Copying,
                    TaskProgress::Bytes {
                        completed: done,
                        total,
                    },
                );
            }
            instance_import::Progress::Extracting => {
                task.update(TaskStage::Extracting, TaskProgress::Indeterminate);
            }
        })
    };
    instance_import::import(plan, name, config, sink).await
}
