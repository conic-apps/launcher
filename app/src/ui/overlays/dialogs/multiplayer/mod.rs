// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The multiplayer dialog: drives [`multiplayer`] and pushes the session into
//! the `MultiplayerState` global. Its poll thread reports through a sink that
//! marshals each notice onto the Slint event loop.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use crate::slint_backend::{App, GameState, MultiplayerPeer, MultiplayerPlayer, MultiplayerState};
use crate::ui::services::report::deliver;
use crate::usecases::generation::Gate;
use multiplayer::{NexusService, PeerInfo, SessionEvent, SessionState};

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
    static CONTROLLER: RefCell<Option<Rc<Controller>>> = const { RefCell::new(None) };
}

pub(crate) struct Controller {
    service: Arc<NexusService>,
    peers_timer: Timer,
    /// The delay before switching to the manager after a finished download.
    switch_timer: Timer,
    initialized: Cell<bool>,
    download_task: RefCell<Option<tokio::task::JoinHandle<()>>>,
    /// The task-centre card for the running library download, -1 when none.
    download_task_id: Cell<i32>,
    /// Issues the download's token and invalidates it on cancel.
    download_gate: Gate,
}
