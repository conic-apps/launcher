// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The multiplayer dialog's "script": drives `slint-multiplayer` and pushes the
//! session into the `MultiplayerState` global
//! (src/store/multiplayer.ts + src/overlays/dialogs/multiplayer/*).
//!
//! The Vue keeps the store in Pinia and lets the Tauri plugin own the session
//! and its event poll thread. Here the poll thread lives in
//! [`multiplayer::NexusService`] and reports every notice to a sink this
//! module installs; the sink marshals the notice onto the Slint event loop,
//! which is where the store's `on(...)` handlers (`handle_event`) run.
//!
//! Everything that touches the disk, the network or the library runs on the
//! app's tokio runtime (`crate::runtime`) and reports back with
//! `upgrade_in_event_loop`. A `Rc<Controller>` cannot cross into a task, so the
//! event-loop half reaches it through the `CONTROLLER` thread-local (the same
//! arrangement `launch.rs` uses for the shared config).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex, atomic::Ordering},
    time::Duration,
};

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel, Weak};

use crate::slint_backend::{App, GameState, MultiplayerPeer, MultiplayerPlayer, MultiplayerState};
use multiplayer::{NexusService, PeerInfo, SessionEvent, SessionState};

/// `NAT_POLL_UNKNOWN_INTERVAL` / `NAT_POLL_KNOWN_INTERVAL` of the Vue store.
const NAT_POLL_UNKNOWN_INTERVAL: Duration = Duration::from_secs(10);
const NAT_POLL_KNOWN_INTERVAL: Duration = Duration::from_secs(60);

thread_local! {
    /// The running controller, for the event-loop half of a task (see the
    /// module comment).
    static CONTROLLER: RefCell<Option<Rc<Controller>>> = const { RefCell::new(None) };
}

struct Controller {
    service: Arc<NexusService>,
    /// `setInterval` behind `schedulePeersPolling`.
    peers_timer: Timer,
    /// The Vue's `setTimeout(..., 500)` after a finished download.
    switch_timer: Timer,
    /// `initialized` in `store/multiplayer.ts`.
    initialized: Cell<bool>,
    /// The running library download (the Tauri command's task handle).
    download_task: RefCell<Option<tokio::task::JoinHandle<()>>>,
}

/// Registers every multiplayer callback on the `MultiplayerState` global.
pub fn setup(ui: &App) {
    // The sink the poll thread reports through. `Weak<App>` is `Send` but not
    // necessarily `Sync`, which the sink type asks for, so it goes behind a
    // mutex.
    let ui_weak = Arc::new(Mutex::new(ui.as_weak()));
    let sink: multiplayer::EventSink = Arc::new(move |event| {
        let weak = ui_weak.lock().map(|weak| weak.clone()).ok();
        if let Some(weak) = weak {
            let _ = weak.upgrade_in_event_loop(move |ui| handle_event(&ui, event));
        }
    });

    let controller = Rc::new(Controller {
        service: Arc::new(NexusService::new(sink)),
        peers_timer: Timer::default(),
        switch_timer: Timer::default(),
        initialized: Cell::new(false),
        download_task: RefCell::new(None),
    });
    CONTROLLER.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&controller)));

    let state = ui.global::<MultiplayerState>();

    // The footer's connect button (`openConnect`).
    {
        let weak = ui.as_weak();
        state.on_open(move || {
            let weak_task = weak.clone();
            crate::runtime::spawn(async move {
                let valid = multiplayer::check_library().await.is_ok();
                let _ = weak_task.upgrade_in_event_loop(move |ui| {
                    let state = ui.global::<MultiplayerState>();
                    state.set_component(
                        (if valid {
                            "multiplayerManager"
                        } else {
                            "downloadDescription"
                        })
                        .into(),
                    );
                    state.set_visible(true);
                });
            });
        });
    }

    // The manager's `onMounted` (`multiplayerStore.init()`).
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_start(move || {
            if controller.initialized.replace(true) {
                return;
            }
            let Some(ui) = weak.upgrade() else { return };
            push_refresh(&ui, &controller);
        });
    }

    // `createRoom(profileName)`.
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_create_room(move || {
            let Some(ui) = weak.upgrade() else { return };
            let player_name = ui
                .global::<GameState>()
                .get_current_account_name()
                .to_string();
            ui.global::<MultiplayerState>().set_has_fault(false);
            let service = Arc::clone(&controller.service);
            crate::runtime::spawn(async move {
                if let Err(error) = service.create_room(Some(player_name.as_str()), None).await {
                    log::error!(target: "multiplayer", "failed to create a room: {error}");
                }
            });
        });
    }

    // `submitJoin` + `joinRoom(code, profileName)`.
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_join_room(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<MultiplayerState>();
            let code = state.get_code_input().trim().to_string();
            if code.is_empty() {
                return;
            }
            let player_name = ui
                .global::<GameState>()
                .get_current_account_name()
                .to_string();
            state.set_has_fault(false);
            let weak_task = weak.clone();
            let service = Arc::clone(&controller.service);
            crate::runtime::spawn(async move {
                match service.join_room(&code, Some(player_name.as_str())).await {
                    Ok(()) => {
                        let _ = weak_task.upgrade_in_event_loop(|ui| {
                            ui.global::<MultiplayerState>().set_code_input_open(false);
                        });
                    }
                    Err(error) => {
                        log::error!(target: "multiplayer", "failed to join a room: {error}");
                    }
                }
            });
        });
    }

    // `leaveRoom()`.
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_leave_room(move || {
            if weak.upgrade().is_none() {
                return;
            }
            let weak_task = weak.clone();
            let service = Arc::clone(&controller.service);
            crate::runtime::spawn(async move {
                let result = service.leave_room().await;
                let _ = weak_task.upgrade_in_event_loop(move |ui| match result {
                    Ok(()) => {
                        let state = ui.global::<MultiplayerState>();
                        state.set_state("waiting".into());
                        state.set_room_code("".into());
                        state.set_players(ModelRc::new(VecModel::from(
                            Vec::<MultiplayerPlayer>::new(),
                        )));
                        state
                            .set_peers(ModelRc::new(VecModel::from(Vec::<MultiplayerPeer>::new())));
                        state.set_has_local_peer(false);
                        state.set_local_nat_code(-1);
                        state.set_url("".into());
                        state.set_has_fault(false);
                        with_controller(|controller| controller.peers_timer.stop());
                    }
                    Err(error) => {
                        log::error!(target: "multiplayer", "failed to leave the room: {error}");
                    }
                });
            });
        });
    }

    // `copyCode()`.
    {
        let weak = ui.as_weak();
        state.on_copy_room_code(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<MultiplayerState>();
            if let Err(error) =
                crate::config_bridge::copy_to_clipboard(state.get_room_code().as_str())
            {
                log::warn!(target: "multiplayer", "failed to write to the clipboard: {error}");
            }
            state.set_code_copied(true);
        });
    }

    // `codeInputValid = isRoomCodeValid(codeInput)`.
    {
        let weak = ui.as_weak();
        state.on_code_input_changed(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<MultiplayerState>();
            let valid = multiplayer::is_room_code_valid(state.get_code_input().as_str());
            state.set_code_input_valid(valid);
        });
    }

    // `DownloadProgress.vue`'s `onMounted` (`downloadTask.start()`).
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_start_download(move || {
            let Some(ui) = weak.upgrade() else { return };
            start_download(&ui, &controller);
        });
    }

    // `cancelDownload()`: stop the task, then fall back to the description.
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_cancel_download(move || {
            cancel_download(&controller);
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<MultiplayerState>();
            state.set_component("downloadDescription".into());
            state.set_visible(false);
        });
    }

    // The dialog's own "close" (the scrim-less footer of a few screens).
    {
        let weak = ui.as_weak();
        state.on_close(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<MultiplayerState>().set_visible(false);
            }
        });
    }
}

/// Stops the session (the original's `RunEvent::Exit` handler): the poll thread
/// is joined and the Conic Nexus session is destroyed before the process goes.
pub fn shutdown() {
    with_controller(|controller| controller.service.shutdown());
}

/// Runs `body` with the controller, if the app has installed one.
fn with_controller(body: impl FnOnce(&Rc<Controller>)) {
    CONTROLLER.with(|slot| {
        let slot = slot.borrow();
        if let Some(controller) = slot.as_ref() {
            body(controller);
        }
    });
}

/// The store's `refresh()`: read the session snapshot and follow it with a peer
/// refresh.
fn push_refresh(ui: &App, controller: &Rc<Controller>) {
    let weak = ui.as_weak();
    let service = Arc::clone(&controller.service);
    crate::runtime::spawn(async move {
        match service.get_session_state().await {
            Ok(session) => {
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    apply_session(&ui, session);
                    with_controller(|controller| schedule_peers_polling(&ui, controller));
                });
            }
            Err(error) => {
                log::warn!(target: "multiplayer", "failed to read the session state: {error}");
            }
        }
    });
}

/// The store's `refreshPeers()`.
fn push_refresh_peers(ui: &App, controller: &Rc<Controller>) {
    let weak = ui.as_weak();
    let service = Arc::clone(&controller.service);
    crate::runtime::spawn(async move {
        let peers = service.query_peers().await.unwrap_or_default();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            apply_peers(&ui, &peers);
            with_controller(|controller| schedule_peers_polling(&ui, controller));
        });
    });
}

/// The store's `schedulePeersPolling()` + `watch(state)`.
fn schedule_peers_polling(ui: &App, controller: &Rc<Controller>) {
    let state = ui.global::<MultiplayerState>();
    let session = state.get_state();
    if session != "host-ok" && session != "guest-ok" {
        controller.peers_timer.stop();
        return;
    }
    // `isNatKnown`: the local peer is there and its NAT code is not 0.
    let known = state.get_has_local_peer() && state.get_local_nat_code() != 0;
    let interval = if known {
        NAT_POLL_KNOWN_INTERVAL
    } else {
        NAT_POLL_UNKNOWN_INTERVAL
    };
    let weak = ui.as_weak();
    let controller_inner = Rc::clone(controller);
    controller
        .peers_timer
        .start(TimerMode::Repeated, interval, move || {
            let Some(ui) = weak.upgrade() else { return };
            push_refresh_peers(&ui, &controller_inner);
        });
}

/// One notice from the poll thread, dispatched like the store's `on(...)`
/// handlers.
fn handle_event(ui: &App, event: SessionEvent) {
    let state = ui.global::<MultiplayerState>();
    match event.r#type {
        // state-changed
        0 => {
            if let Some(value) = event.payload.get("state") {
                state.set_state(state_of(value).into());
            }
            with_controller(|controller| push_refresh(ui, controller));
        }
        // player-joined / player-left
        1 | 2 => with_controller(|controller| push_refresh(ui, controller)),
        // host-ready
        3 => {
            if let Some(room) = event.payload.get("room").and_then(|value| value.as_str()) {
                state.set_room_code(room.into());
            }
            with_controller(|controller| push_refresh(ui, controller));
        }
        // guest-ready
        4 => {
            if let Some(url) = event.payload.get("url").and_then(|value| value.as_str()) {
                state.set_url(url.into());
            }
            with_controller(|controller| push_refresh(ui, controller));
        }
        // fault
        5 => {
            let code = event
                .payload
                .get("code")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            let message = event
                .payload
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            state.set_has_fault(true);
            state.set_fault_code(code as i32);
            state.set_fault_message(message.into());
            state.set_state("exception".into());
        }
        _ => {}
    }
}

/// Applies a session snapshot (`refresh`'s body).
fn apply_session(ui: &App, session: SessionState) {
    let state = ui.global::<MultiplayerState>();
    state.set_state(session.state.into());
    state.set_room_code(session.room_code.into());

    let detail = &session.detail;
    if let Some(profiles) = detail.get("profiles").and_then(|value| value.as_array()) {
        let players: Vec<MultiplayerPlayer> = profiles
            .iter()
            .map(|profile| {
                let text = |key: &str| {
                    profile
                        .get(key)
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .into()
                };
                MultiplayerPlayer {
                    machine_id: text("machine_id"),
                    name: text("name"),
                    vendor: text("vendor"),
                    kind: text("kind"),
                }
            })
            .collect();
        state.set_players(ModelRc::new(VecModel::from(players)));
    }
    if let Some(url) = detail.get("url").and_then(|value| value.as_str()) {
        state.set_url(url.into());
    }
    if let Some(error) = detail.get("error") {
        state.set_has_fault(true);
        state.set_fault_code(
            error
                .get("code")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default() as i32,
        );
        state.set_fault_message(
            error
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .into(),
        );
    }
}

/// Applies a peer list, including the local NAT code the header reads.
fn apply_peers(ui: &App, peers: &[PeerInfo]) {
    let mut has_local = false;
    let mut local_nat = -1;
    let rows: Vec<MultiplayerPeer> = peers
        .iter()
        .map(|peer| {
            if peer.is_local {
                has_local = true;
                local_nat = peer.nat;
            }
            MultiplayerPeer {
                hostname: peer.hostname.clone().into(),
                ipv4: peer.ipv4.clone().into(),
                is_local: peer.is_local,
                nat: peer.nat,
            }
        })
        .collect();
    let state = ui.global::<MultiplayerState>();
    state.set_peers(ModelRc::new(VecModel::from(rows)));
    state.set_has_local_peer(has_local);
    state.set_local_nat_code(local_nat);
}

/// `toStateName(state)`: the library reports the state as a name already, but
/// the FFI event may carry the numeric code instead.
fn state_of(value: &serde_json::Value) -> String {
    if let Some(name) = value.as_str() {
        return name.to_string();
    }
    match value.as_i64() {
        Some(0) => "waiting",
        Some(1) => "host-scanning",
        Some(2) => "host-starting",
        Some(3) => "host-ok",
        Some(4) => "guest-connecting",
        Some(5) => "guest-starting",
        Some(6) => "guest-ok",
        Some(7) => "exception",
        _ => "unknown",
    }
    .to_string()
}

/// `ConicNexusLibraryDownloadTask` + the progress `Channel` the Tauri command
/// fed: the download runs on the runtime while a 50ms ticker reports the shared
/// `DownloadState`.
fn start_download(ui: &App, controller: &Rc<Controller>) {
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
fn cancel_download(controller: &Rc<Controller>) {
    if let Some(task) = controller.download_task.borrow_mut().take() {
        task.abort();
    }
}

/// One tick of the download's progress.
fn push_download(weak: &Weak<App>, progress: &download::progress::DownloadState) {
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
    let value_text = format_bytes(value);
    let max_text = format_bytes(max);
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
fn finish_download(ui: &App, ok: bool) {
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

/// The Vue frontend's `formatBytes` (`crates/download/index.ts`).
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.2} {}", UNITS[unit])
}
