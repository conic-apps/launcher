// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's install producer: the one place an instance id becomes a
//! `GameInstall` task.
//!
//! The task centre decides when the work runs — a second instance's install
//! waits in the exclusive lane — and owns the card and the cancel policy. What
//! stays here is the work itself: run the installer, mirror its progress into the
//! strip beside the button, and re-read the listing when it ends so `.install.lock`
//! turns the button from “Download” into “Launch”.

use std::sync::Arc;

use super::*;
use crate::usecases::tasks::{TaskHandle, TaskWork};
use config::Config;
use shared::task::{TaskKind, TaskSpec, TaskTarget};

/// Hooks the button's state to the registry: every task event re-projects it.
pub(crate) fn watch() {
    crate::ui::services::tasks::observe(project);
}

/// Asks for an install of `instance_id`, queueing it when the exclusive lane is
/// busy. Also the repair path: the caller drops `.install.lock` first, so the
/// same work runs again over whatever is already on disk.
pub(crate) fn start(ui: &App, instance_id: String) {
    let service = crate::ui::services::tasks::service();
    if service
        .active_on(TaskKind::GameInstall, &instance_id)
        .is_some()
    {
        return;
    }
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        // `instance.toml` is read on the runtime; the submission itself runs on
        // the event loop, where the centre lives.
        let Some(instance) = instance::get_instance_by_id(&instance_id).await else {
            log::error!("failed to read instance '{instance_id}' for the install");
            return;
        };
        crate::ui::services::report::report(&weak, move |ui| enqueue(&ui, instance));
    });
}

/// Re-derives the launch button's state from the registry.
///
/// Called after every task event and on every apply of the game view, so the
/// button follows a task that was cancelled, finished or queued elsewhere in the
/// app — the UI owns no copy of the scheduler's state.
pub(crate) fn project(ui: &App) {
    let state = ui.global::<GameState>();
    let id = state.get_current_id().to_string();
    if !state.get_has_current() || id.is_empty() {
        state.set_current_task("idle".into());
        state.set_task_id(-1);
        install_progress::clear(&state);
        return;
    }
    // The game view is built before the task service in `main`, so the first
    // paint has nothing to read and says so.
    let Some(service) = crate::ui::services::tasks::service_if_ready() else {
        state.set_current_task("idle".into());
        state.set_task_id(-1);
        install_progress::clear(&state);
        return;
    };
    // Only an install is a task. A launch has its own page and its own cancel
    // button, so nothing about it belongs on this button.
    let Some(task) = service.active_on(TaskKind::GameInstall, &id) else {
        state.set_current_task("idle".into());
        state.set_task_id(-1);
        // Nothing is running for this instance, so whatever the strip last showed
        // belongs to work that has ended or to another instance.
        install_progress::clear(&state);
        return;
    };
    state.set_current_task(
        match task.state {
            shared::task::TaskState::Queued => "queued",
            shared::task::TaskState::Running => "running",
        }
        .into(),
    );
    state.set_task_id(task.id);
}

fn enqueue(ui: &App, instance: Instance) {
    let service = crate::ui::services::tasks::service();
    if service
        .active_on(TaskKind::GameInstall, &instance.id)
        .is_some()
    {
        return;
    }
    let config = {
        let config = Rc::clone(&controller().borrow().config);
        config.borrow().clone()
    };
    let loader = instance
        .config
        .runtime
        .mod_loader_type
        .as_ref()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    let spec = TaskSpec {
        kind: TaskKind::GameInstall,
        subject: instance.config.name.clone(),
        context: instance.config.runtime.minecraft.clone(),
        target: TaskTarget::Instance {
            instance_id: instance.id.clone(),
        },
    };
    let work: TaskWork = {
        let weak = ui.as_weak();
        Box::new(move |task| {
            let weak = weak.clone();
            let instance = instance.clone();
            let config = config.clone();
            let loader = loader.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| run(&ui, config, instance, loader, task));
        })
    };
    service.submit(spec, work);
}

/// The task was admitted: spawn the installer and mirror its progress.
fn run(ui: &App, config: Config, instance: Instance, loader: String, task: TaskHandle) {
    let weak = ui.as_weak();
    let sink: ::install::InstallSink = {
        let weak = weak.clone();
        let task = task.clone();
        Arc::new(move |progress| {
            let (stage, snapshot) = crate::usecases::progress::install(&progress);
            task.update(stage, snapshot);
            let view = install_progress::install_view(&progress, &loader);
            crate::ui::services::report::report(&weak, move |ui| {
                install_progress::apply(&ui.global::<GameState>(), &view);
            });
        })
    };
    let completion = task.clone();
    let handle = crate::support::runtime::spawn({
        let weak = weak.clone();
        async move {
            match ::install::install(config, instance, sink).await {
                Ok(()) => completion.succeed(),
                Err(error) => {
                    log::error!("failed to install the game: {error}");
                    completion.fail(error.to_string());
                }
            }
            // The lock file is what the listing reads as "installed", so the
            // button can only flip after a reload.
            crate::ui::services::report::report(&weak, |ui| GameController::reload(&ui));
        }
    });
    let abort = handle.abort_handle();
    crate::ui::services::tasks::service().attach_cancel(
        task.id(),
        Arc::new(move || {
            abort.abort();
        }),
    );
}
