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
            crate::ui::services::report::report(&weak, move |ui| handle_event(&ui, event));
        }
    });

    let controller = Rc::new(Controller {
        service: Arc::new(NexusService::new(sink)),
        peers_timer: Timer::default(),
        switch_timer: Timer::default(),
        initialized: Cell::new(false),
        download_task: RefCell::new(None),
        download_gate: Gate::new(),
    });
    CONTROLLER.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&controller)));

    setup_session_actions(ui, Rc::clone(&controller));
    setup_code_actions(ui);
    setup_download_actions(ui, Rc::clone(&controller));
}

/// Opening, joining, creating and leaving a room.
pub(crate) fn setup_session_actions(ui: &App, controller: Rc<Controller>) {
    let state = ui.global::<MultiplayerState>();
    // The footer's connect button.
    {
        let weak = ui.as_weak();
        state.on_open(move || {
            let weak_task = weak.clone();
            crate::support::runtime::spawn(async move {
                let valid = crate::usecases::multiplayer::library_ready().await;
                crate::ui::services::report::report(&weak_task, move |ui| {
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
    // The manager's one-time initialization.
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
    // Create a room with the current profile name.
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
            crate::support::runtime::spawn(async move {
                if let Err(error) =
                    crate::usecases::multiplayer::create_room(&service, &player_name).await
                {
                    log::error!(target: "multiplayer", "failed to create a room: {error}");
                }
            });
        });
    }
    // Join a room with the entered code and the current profile name.
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
            crate::support::runtime::spawn(async move {
                match crate::usecases::multiplayer::join_room(&service, &code, &player_name).await {
                    Ok(()) => {
                        crate::ui::services::report::report(&weak_task, |ui| {
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
    // Leave the room and reset the session state.
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_leave_room(move || {
            if weak.upgrade().is_none() {
                return;
            }
            let weak_task = weak.clone();
            let service = Arc::clone(&controller.service);
            crate::support::runtime::spawn(async move {
                let result = crate::usecases::multiplayer::leave_room(&service).await;
                crate::ui::services::report::report(&weak_task, move |ui| match result {
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
    // Copy the room code to the clipboard.
    {
        let weak = ui.as_weak();
        state.on_copy_room_code(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<MultiplayerState>();
            if let Err(error) =
                crate::ui::services::app_config::copy_to_clipboard(state.get_room_code().as_str())
            {
                log::warn!(target: "multiplayer", "failed to write to the clipboard: {error}");
            }
            state.set_code_copied(true);
        });
    }
    // Recompute the room code's validity as it changes.
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
    // The download screen's startup.
    {
        let weak = ui.as_weak();
        let controller = Rc::clone(&controller);
        state.on_start_download(move || {
            let Some(ui) = weak.upgrade() else { return };
            start_download(&ui, &controller);
        });
    }
    // Stop the task, then fall back to the description.
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

/// Stops the session at exit: the poll thread is joined and the Conic Nexus
/// session is destroyed before the process goes.
pub fn shutdown() {
    with_controller(|controller| controller.service.shutdown());
}
