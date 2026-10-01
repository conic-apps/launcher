// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Conic Nexus library download screen.

use super::*;

/// `ConicNexusLibraryDownloadTask` + the progress `Channel` the Tauri command
/// fed: the download runs on the runtime while a 50ms ticker reports the shared
/// `DownloadState`.
pub(crate) fn start_download(ui: &App, controller: &Rc<Controller>) {
    cancel_download(controller);

    // The screen's initial `ref`s (`progressBar`, `ProgressPhase.Prepare`).
    {
        let state = ui.global::<MultiplayerState>();
        state.set_download_phase("prepare".into());
        state.set_download_loading(true);
        state.set_download_value(0.0);
        state.set_download_max(0.0);
        state.set_download_value_text("".into());
        state.set_download_max_text("".into());
    }

    let progress = download::progress::DownloadState::default();
    let weak = ui.as_weak();
    let task = crate::runtime::spawn({
        let progress = progress.clone();
        async move {
            let future = multiplayer::download_library(&progress);
            tokio::pin!(future);
            let mut ticker = tokio::time::interval(Duration::from_millis(50));
            loop {
                tokio::select! {
                    result = &mut future => {
                        let ok = result.is_ok();
                        if let Err(error) = result {
                            log::error!(target: "multiplayer", "failed to download the library: {error}");
                        }
                        let _ = weak.upgrade_in_event_loop(move |ui| finish_download(&ui, ok));
                        break;
                    }
                    _ = ticker.tick() => {
                        push_download(&weak, &progress);
                    }
                }
            }
        }
    });
    *controller.download_task.borrow_mut() = Some(task);
}

/// `cancelDownloadHandle()`.
pub(crate) fn cancel_download(controller: &Rc<Controller>) {
    if let Some(task) = controller.download_task.borrow_mut().take() {
        task.abort();
    }
}

/// One tick of the download's progress.
pub(crate) fn push_download(weak: &Weak<App>, progress: &download::progress::DownloadState) {
    let phase = progress
        .phase
        .lock()
        .map(|phase| phase.clone())
        .unwrap_or_default();
    let completed = progress.completed_bytes.load(Ordering::SeqCst);
    let total = progress.total_bytes.load(Ordering::SeqCst);
    let (name, loading, value, max) = match phase {
        download::progress::DownloadPhase::VerifyExistingFiles => ("prepare", true, 0, 10),
        download::progress::DownloadPhase::DownloadFiles => (
            if total == 0 { "prepare" } else { "downloading" },
            total == 0,
            completed,
            total,
        ),
    };
    let value_text = crate::formatting::bytes(value);
    let max_text = crate::formatting::bytes(max);
    let _ = weak.upgrade_in_event_loop(move |ui| {
        let state = ui.global::<MultiplayerState>();
        state.set_download_phase(name.into());
        state.set_download_loading(loading);
        state.set_download_value(value as f32);
        state.set_download_max(max as f32);
        state.set_download_value_text(value_text.into());
        state.set_download_max_text(max_text.into());
    });
}

/// The tail of `DownloadProgress.vue`'s `onMounted`: the finished bar, then the
/// switch to the manager half a second later.
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
