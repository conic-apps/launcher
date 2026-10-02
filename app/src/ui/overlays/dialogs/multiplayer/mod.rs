// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The multiplayer dialog's "script": drives [`multiplayer`] and pushes the
//! session into the `MultiplayerState` global.
//!
//! The poll thread lives in [`multiplayer::NexusService`] and reports every
//! notice to a sink this module installs; the sink marshals the notice onto the
//! Slint event loop, which is where `handle_event` runs.
//!
//! Everything that touches the disk, the network or the library runs on the
//! app's tokio runtime (`crate::support::runtime`) and reports back with
//! `upgrade_in_event_loop`. A `Rc<Controller>` cannot cross into a task, so the
//! event-loop half reaches it through the `CONTROLLER` thread-local (the same
//! arrangement the launch module uses for its controller).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use crate::slint_backend::{App, GameState, MultiplayerPeer, MultiplayerPlayer, MultiplayerState};
use crate::ui::services::report::{Gate, deliver};
use multiplayer::{NexusService, PeerInfo, SessionEvent, SessionState};

/// How often to poll peers when the local NAT code is unknown / known.
pub(crate) const NAT_POLL_UNKNOWN_INTERVAL: Duration = Duration::from_secs(10);
pub(crate) const NAT_POLL_KNOWN_INTERVAL: Duration = Duration::from_secs(60);

mod events;
mod library;
mod presenter;
mod wiring;

pub(crate) use events::*;
pub(crate) use library::*;
pub(crate) use presenter::*;
pub(crate) use wiring::*;

thread_local! {
    /// The running controller, for the event-loop half of a task (see the
    /// module comment).
    static CONTROLLER: RefCell<Option<Rc<Controller>>> = const { RefCell::new(None) };
}

pub(crate) struct Controller {
    service: Arc<NexusService>,
    /// The peer poll timer.
    peers_timer: Timer,
    /// The half-second delay before switching to the manager after a finished
    /// download.
    switch_timer: Timer,
    /// Whether the manager has been initialized.
    initialized: Cell<bool>,
    /// The running library download.
    download_task: RefCell<Option<tokio::task::JoinHandle<()>>>,
    /// Issues the library download's token and invalidates it on cancel.
    download_gate: Gate,
}
