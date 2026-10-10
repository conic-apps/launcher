// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Conic Nexus library download screen.

use super::*;

/// Runs the library download. The `multiplayer` crate samples its own progress
/// and reports through the sink built here; only the abort handle and the
/// `Gate` are the app's.
pub(crate) fn start_download(ui: &App, controller: &Rc<Controller>) {
    cancel_download(controller);
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

    let weak = ui.as_weak();
    let sink: multiplayer::LibrarySink = {
        let weak = weak.clone();
        let token = token.clone();
        Arc::new(move |snapshot| {
            let view = download_view(&snapshot);
            deliver(&weak, &token, move |ui| {
                view.apply(&ui.global::<MultiplayerState>())
            });
        })
    };
    let task = crate::support::runtime::spawn({
        let weak = weak.clone();
        let token = token.clone();
        async move {
            let result = multiplayer::download_library(sink).await;
            if let Err(error) = &result {
                log::error!(target: "multiplayer", "failed to download the library: {error}");
            }
            deliver(&weak, &token, move |ui| {
                finish_download(&ui, result.is_ok())
            });
        }
    });
    *controller.download_task.borrow_mut() = Some(task);
}

/// Stops the running download task and stops its in-flight reports from drawing.
pub(crate) fn cancel_download(controller: &Rc<Controller>) {
    controller.download_gate.invalidate();
    if let Some(task) = controller.download_task.borrow_mut().take() {
        task.abort();
    }
}

pub(crate) fn finish_download(ui: &App, ok: bool) {
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
