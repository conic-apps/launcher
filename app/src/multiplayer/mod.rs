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
pub(crate) const NAT_POLL_UNKNOWN_INTERVAL: Duration = Duration::from_secs(10);
pub(crate) const NAT_POLL_KNOWN_INTERVAL: Duration = Duration::from_secs(60);

mod events;
mod library;
mod wiring;

pub(crate) use events::*;
pub(crate) use library::*;
pub(crate) use wiring::*;

thread_local! {
    /// The running controller, for the event-loop half of a task (see the
    /// module comment).
    static CONTROLLER: RefCell<Option<Rc<Controller>>> = const { RefCell::new(None) };
}

pub(crate) struct Controller {
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
