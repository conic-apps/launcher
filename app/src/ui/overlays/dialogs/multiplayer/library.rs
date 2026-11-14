// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Conic Nexus library download screen.

use super::*;
use crate::usecases::generation::Token;
use crate::usecases::tasks::{TaskHandle, TaskWork};
use shared::task::{TaskKind, TaskSpec, TaskTarget};

/// Runs the library download. The `multiplayer` crate samples its own progress and
/// reports through the sink built here; the task centre owns the card, the
/// scheduling and the cancel policy. The download sits in the parallel lane, so it
/// is admitted at once unless a task of its own kind is already queued ahead of it.
pub(crate) fn start_download(ui: &App, controller: &Rc<Controller>) {
    abort_download(controller);
    let token = controller.download_gate.issue();

    {
        let state = ui.global::<MultiplayerState>();
        state.set_download_phase("prepare".into());
        state.set_download_loading(true);
        state.set_download_value(0.0);
        state.set_download_max(0.0);
        state.set_download_value_text("".into());
        state.set_download_max_text("".into());
    }

    let service = crate::ui::services::tasks::service();
    let spec = TaskSpec {
        kind: TaskKind::LibraryDownload,
        subject: "Conic Nexus".into(),
        context: String::new(),
        target: TaskTarget::MultiplayerLibrary,
    };
    // The work needs the controller, which is a thread-local; the closure has to
    // be `Send`, so it hops back onto the event loop first.
    let work: TaskWork = {
        let weak = ui.as_weak();
        Box::new(move |task| {
            let weak = weak.clone();
            let token = token.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| admitted(&ui, task, token));
        })
    };
    service.submit(spec, work);
}

/// The download was admitted: spawn it and wire its cancel hook.
fn admitted(ui: &App, task: TaskHandle, token: Token) {
    let id = task.id();
    with_controller(|controller| controller.download_task_id.set(id));

    let weak = ui.as_weak();
    let sink: multiplayer::LibrarySink = {
        let weak = weak.clone();
        let token = token.clone();
        let task = task.clone();
        Arc::new(move |snapshot| {
            let (stage, progress) = crate::usecases::progress::download(&snapshot);
            task.update(stage, progress);
            let view = download_view(&snapshot);
            deliver(&weak, &token, move |ui| {
                view.apply(&ui.global::<MultiplayerState>())
            });
        })
    };
    let completion = task.clone();
    let handle = crate::support::runtime::spawn({
        let weak = weak.clone();
        let token = token.clone();
        async move {
            let result = multiplayer::download_library(sink).await;
            match &result {
                Ok(()) => completion.succeed(),
                Err(error) => {
                    completion.fail(error.to_string());
                    log::error!(target: "multiplayer", "failed to download the library: {error}");
                }
            }
            deliver(&weak, &token, move |ui| {
                finish_download(&ui, result.is_ok())
            });
        }
    });
    let abort = handle.abort_handle();
    with_controller(|controller| {
        let gate = controller.download_gate.clone();
        crate::ui::services::tasks::service().attach_cancel(
            id,
            Arc::new(move || {
                gate.invalidate();
                abort.abort();
            }),
        );
        *controller.download_task.borrow_mut() = Some(handle);
    });
}

/// Stops the running download without the task centre's confirmation prompt.
/// Used when a new download replaces one that is already running.
fn abort_download(controller: &Rc<Controller>) {
    controller.download_gate.invalidate();
    if let Some(task) = controller.download_task.borrow_mut().take() {
        task.abort();
    }
    let id = controller.download_task_id.replace(-1);
    if id >= 0 {
        crate::ui::services::tasks::service().cancel_now(id);
    }
}

/// The dialog's "Stop download" button. It goes through the same ten-second
/// rule as the task card's X, so the two paths cannot disagree.
pub(crate) fn cancel_download(ui: &App, controller: &Rc<Controller>) {
    let id = controller.download_task_id.get();
    if id >= 0 {
        ui.global::<crate::slint_backend::TaskState>()
            .invoke_request_cancel(id);
    } else {
        abort_download(controller);
    }
}

pub(crate) fn finish_download(ui: &App, ok: bool) {
    with_controller(|controller| {
        controller.download_task_id.set(-1);
        controller.download_task.borrow_mut().take();
    });
    let state = ui.global::<MultiplayerState>();
    if !ok {
        return;
    }
    state.set_download_phase("finished".into());
    state.set_download_loading(false);
    state.set_download_value(state.get_download_max());
    state.set_download_value_text(state.get_download_max_text());

    with_controller(|controller| {
        let weak = ui.as_weak();
        controller.switch_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(500),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.global::<MultiplayerState>()
                        .set_component("multiplayerManager".into());
                }
            },
        );
    });
}
