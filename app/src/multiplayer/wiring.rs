// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The multiplayer callbacks, registered on `MultiplayerState`.

use super::*;

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

    setup_session_actions(ui, Rc::clone(&controller));
    setup_code_actions(ui);
    setup_download_actions(ui, Rc::clone(&controller));
}

/// Opening, joining, creating and leaving a room.
pub(crate) fn setup_session_actions(ui: &App, controller: Rc<Controller>) {
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
}

/// The room code's copy button and its validity test.
pub(crate) fn setup_code_actions(ui: &App) {
    let state = ui.global::<MultiplayerState>();
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
}

/// The library download screen: start, cancel, and the dialog's close.
pub(crate) fn setup_download_actions(ui: &App, controller: Rc<Controller>) {
    let state = ui.global::<MultiplayerState>();
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
