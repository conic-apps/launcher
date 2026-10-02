// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The session and peer events the poll thread reports, and the pushes
//! they trigger.

use super::*;

/// Runs `body` with the controller, if the app has installed one.
pub(crate) fn with_controller(body: impl FnOnce(&Rc<Controller>)) {
    CONTROLLER.with(|slot| {
        let slot = slot.borrow();
        if let Some(controller) = slot.as_ref() {
            body(controller);
        }
    });
}

/// Reads the session snapshot and (re)schedules the peer poll.
pub(crate) fn push_refresh(ui: &App, controller: &Rc<Controller>) {
    let weak = ui.as_weak();
    let service = Arc::clone(&controller.service);
    crate::support::runtime::spawn(async move {
        match service.get_session_state().await {
            Ok(session) => {
                crate::ui::services::report::report(&weak, move |ui| {
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

/// Reads the peer list.
pub(crate) fn push_refresh_peers(ui: &App, controller: &Rc<Controller>) {
    let weak = ui.as_weak();
    let service = Arc::clone(&controller.service);
    crate::support::runtime::spawn(async move {
        let peers = service.query_peers().await.unwrap_or_default();
        crate::ui::services::report::report(&weak, move |ui| {
            apply_peers(&ui, &peers);
            with_controller(|controller| schedule_peers_polling(&ui, controller));
        });
    });
}

/// Schedules or stops the peer poll according to the session state.
pub(crate) fn schedule_peers_polling(ui: &App, controller: &Rc<Controller>) {
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

/// One notice from the poll thread, dispatched to the view.
pub(crate) fn handle_event(ui: &App, event: SessionEvent) {
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

/// Applies a session snapshot to the view.
pub(crate) fn apply_session(ui: &App, session: SessionState) {
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
pub(crate) fn apply_peers(ui: &App, peers: &[PeerInfo]) {
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

/// The library reports the state as a name already, but the FFI event may carry
/// the numeric code instead.
pub(crate) fn state_of(value: &serde_json::Value) -> String {
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
