// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's install producer: the one place an instance id becomes a
//! `GameInstall` task.
//!
//! The task centre decides when the work runs and owns the card and the cancel
//! policy; here is the work itself — run the installer, mirror its progress into
//! the strip beside the button, and re-read the listing when it ends so
//! `.install.lock` turns the button from “Download” into “Launch”.
//!
//! [`project`] is the button's whole state, and it reads two registries: the task
//! centre's install and the launch crate's running sessions.

use std::sync::Arc;

use super::*;
use crate::usecases::tasks::{TaskHandle, TaskWork};
use config::Config;
use shared::task::{TaskKind, TaskSpec, TaskTarget};

/// Hooks the button's state to both registries it mirrors: every task event and
/// every session event re-projects it.
pub(crate) fn watch() {
    crate::ui::services::tasks::observe(project);
    crate::ui::services::sessions::observe(|ui, _| project(ui));
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

/// The repair path: drop `.install.lock` so the listing reads the instance as
/// uninstalled, then queue the install again over what is already on disk.
///
/// Shared by the instance settings and the instance list's right-click menu, so
/// the two can never disagree about what repairing does.
pub(crate) fn repair(ui: &App, instance_id: String) {
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        if let Err(error) = instance::remove_install_lock(&instance_id).await {
            log::error!("failed to remove the install lock: {error}");
            return;
        }
        // The listing reads the lock as "installed" and the button follows the
        // listing; the install itself queues behind whatever holds the
        // exclusive lane.
        crate::ui::services::report::report(&weak, move |ui| {
            ui.global::<GameState>().invoke_refresh();
            start(&ui, instance_id);
        });
    });
}

/// The task centre's view of one instance's install: the launch button's three
/// states and the task id that stops it.
///
/// One projection for both readers — the current instance's button and the
/// right-click menu's target, which need not be the same instance.
pub(crate) fn task_of(instance_id: &str) -> (&'static str, i32) {
    // The game view is built before the task service in `main`, so the first
    // paint has nothing to read and says so.
    let Some(service) = crate::ui::services::tasks::service_if_ready() else {
        return ("idle", -1);
    };
    // Only an install is a task. A launch has its own page and its own cancel
    // button, so nothing about it belongs on these.
    let Some(task) = service.active_on(TaskKind::GameInstall, instance_id) else {
        return ("idle", -1);
    };
    let kind = match task.state {
        shared::task::TaskState::Queued => "queued",
        shared::task::TaskState::Running => "running",
    };
    (kind, task.id)
}

/// Re-derives the launch button's state from the registries, after every task and
/// session event and on every apply of the game view — the UI owns no copy of
/// either registry.
pub(crate) fn project(ui: &App) {
    let state = ui.global::<GameState>();
    let id = state.get_current_id().to_string();
    let current = state.get_has_current() && !id.is_empty();
    let (task, task_id) = if current { task_of(&id) } else { ("idle", -1) };
    state.set_current_task(task.into());
    state.set_task_id(task_id);
    // A launch is not a task and has already left the centre by the time its
    // process is up, so the running bit can only come from the session registry.
    state.set_current_running(current && launch::running::is_running(&id));
    if task_id < 0 {
        // Nothing is running for this instance, so whatever the strip last showed
        // belongs to work that has ended or to another instance.
        install_progress::clear(&state);
    }
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
