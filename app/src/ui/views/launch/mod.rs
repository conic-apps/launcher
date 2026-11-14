// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch view's adapter: it waits for the task centre to go idle, holds the
//! exclusive lane while the launch runs, and turns the [`LaunchUpdate`]s into
//! `LaunchState`, gated by a [`Token`] so a cancelled or superseded run cannot
//! draw.
//!
//! A launch is not a task: it has its own screen and cancel button, so it waits
//! for the centre to go idle and then takes the lane
//! (`TaskCenter::reserve_exclusive`) rather than joining the queue.
//!
//! Cancelling is not only dropping the flow: once the crate has spawned the game,
//! the process outlives the task, so [`LaunchController::cancel`] stops the
//! session too. That is why the page's cancel button is never disabled.

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
use launch::running;
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

    /// The one controller, for use on the UI thread. It lives in a thread-local
    /// because an `upgrade_in_event_loop` closure has to be `Send`, and the
    /// controller cannot cross into one.
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
    /// The instance the pending or running launch belongs to. It outlives
    /// `pending` because a cancel must still name the process it has to stop.
    instance_id: Option<String>,
    /// The pids already running for that instance when the launch was requested.
    /// A cancel stops only what this launch started: the instance may have had a
    /// game of its own before, and leaving the page must not kill that one.
    running_before: Vec<u32>,
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
        self.instance_id = Some(instance.id.clone());
        self.running_before = running::running_instances()
            .into_iter()
            .filter(|session| session.id == instance.id)
            .map(|session| session.pid)
            .collect();
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
        // The game stays as it is: this launch is over, not cancelled.
        self.instance_id = None;
        self.running_before.clear();
    }

    /// Cancels the waiting request or the running flow, and stops the process
    /// this launch already spawned. Returns whether there was anything to
    /// cancel.
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
        if busy {
            self.stop_session();
        }
        busy
    }

    /// Stops the session this launch started, when the game is already up: aborting
    /// the task does not end the process, which the crate registered as soon as it
    /// spawned it. The pids recorded at request time are skipped, so an older game
    /// of the same instance is left alone.
    fn stop_session(&mut self) {
        let Some(id) = self.instance_id.take() else {
            return;
        };
        let before = std::mem::take(&mut self.running_before);
        for session in running::running_instances() {
            if session.id == id && !before.contains(&session.pid) {
                log::info!(
                    target: "launch",
                    "cancelling the launch stops the process {} of '{id}'",
                    session.pid
                );
                running::terminate(session.pid);
            }
        }
    }
}
