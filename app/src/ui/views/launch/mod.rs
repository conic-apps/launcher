// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch view's adapter: it waits for the task centre to go idle, holds the
//! exclusive lane while the launch runs, and turns the [`LaunchUpdate`]s into
//! `LaunchState`, gated by a [`Token`] so a cancelled or superseded run cannot
//! draw.
//!
//! A launch is not a task: it has its own screen and its own cancel button, so a
//! card in the queue would be a second, worse copy of the same thing. It waits
//! instead — the page says so — and then takes the lane
//! (`TaskCenter::reserve_exclusive`) rather than joining it, which is what keeps
//! an install from starting under a launching instance.

use std::{cell::RefCell, rc::Rc};

use slint::{ComponentHandle, Weak};

use crate::slint_backend::{App, Dialogs, GameState, LaunchState, Navigation};
use crate::ui::services::report::deliver;
use crate::usecases::generation::{Gate, Token};
use crate::usecases::launch::{LaunchDialog, LaunchUpdate};
use crate::usecases::tasks::TaskReservation;
use account::Account;
use config::Config;
use instance::Instance;
use shared::Sink;

mod ports;
mod presenter;
mod view;
mod wiring;

pub(crate) use ports::*;
pub(crate) use view::*;
pub(crate) use wiring::*;

thread_local! {
    /// The app's shared config, for the event-loop half of the account refresh.
    /// The runtime task cannot hold the `Rc<RefCell<…>>` (it is neither `Send`
    /// nor `Sync`); the use case reports the refreshed account back, and the
    /// event-loop closure writes it through here (and persists it).
    static SHARED_CONFIG: RefCell<Option<Rc<RefCell<Config>>>> = const { RefCell::new(None) };

    /// The one controller, for use on the UI thread.
    ///
    /// It lives in a thread-local rather than in a value `setup` owns because an
    /// `upgrade_in_event_loop` closure has to be `Send`, and the `Rc<RefCell<…>>`
    /// cannot cross into one. Slint's callbacks and the event-loop closures both
    /// run on the UI thread, so both reach it through [`launch_controller`]; the
    /// async task carries only owned values and a `Weak<App>`.
    static CONTROLLER: RefCell<Option<Rc<RefCell<LaunchController>>>> =
        const { RefCell::new(None) };
}

/// Named apart from `setup`'s own `controller` binding, which would shadow it.
pub(crate) fn launch_controller() -> Rc<RefCell<LaunchController>> {
    CONTROLLER
        .with(|cell| cell.borrow().clone())
        .expect("the launch controller is set up before any of its callbacks run")
}

/// One launch, from the request to the game process.
///
/// A run's [`Token`] is invalidated when it is cancelled or superseded, so a late
/// report from a dead run is dropped instead of writing into a fresh one.
#[derive(Default)]
pub(crate) struct LaunchController {
    /// The request waiting for the centre to go idle. `None` once the flow runs.
    pending: Option<(Instance, Config)>,
    /// The flow's handle. `None` while the launch is still waiting, which is
    /// exactly the window in which nothing has been started to abort.
    task: Option<tokio::task::JoinHandle<()>>,
    /// The exclusive lane, held for as long as the flow runs. Dropping it is what
    /// lets the queued installs start again.
    reservation: Option<TaskReservation>,
    gate: Gate,
}

impl LaunchController {
    fn new() -> Self {
        Self::default()
    }

    /// Whether anything at all is in flight — waiting, running or already settled.
    fn busy(&self) -> bool {
        self.pending.is_some() || self.task.is_some() || self.reservation.is_some()
    }

    fn begin(&mut self) -> Token {
        self.gate.issue()
    }

    fn wait(&mut self, instance: Instance, config: Config) {
        self.pending = Some((instance, config));
    }

    fn take_pending(&mut self) -> Option<(Instance, Config)> {
        self.pending.take()
    }

    /// The flow has been spawned: it owns the lane from here on.
    fn running(&mut self, task: tokio::task::JoinHandle<()>, reservation: TaskReservation) {
        self.task = Some(task);
        self.reservation = Some(reservation);
    }

    /// The flow reported its own ending and the page is showing it. The lane is
    /// given back at once: an error page is not work, and queued installs should
    /// not wait for the player to close it.
    fn settle(&mut self) {
        self.task = None;
        self.reservation = None;
    }

    /// Cancels the waiting request or the running flow. Returns whether there was
    /// anything to cancel.
    fn cancel(&mut self) -> bool {
        let busy = self.busy();
        self.gate.invalidate();
        self.pending = None;
        if let Some(task) = self.task.take() {
            task.abort();
        }
        // Dropping the guard hands the exclusive lane back and admits whatever
        // queued behind the launch.
        self.reservation = None;
        busy
    }
}
