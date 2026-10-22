// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch view's adapter: it runs the [`crate::usecases::launch`] use case
//! on the app's runtime and turns its [`LaunchUpdate`]s into `LaunchState`.
//!
//! The use case is UI-neutral; this module builds the output port it reports
//! through, gated by a [`Token`] so a cancelled or superseded run cannot draw.
//! Cancelling aborts the task.

use std::{cell::RefCell, rc::Rc};

use slint::{ComponentHandle, Weak};

use crate::slint_backend::{App, Dialogs, GameState, LaunchState, Navigation};
use crate::ui::services::report::deliver;
use crate::usecases::generation::{Gate, Token};
use crate::usecases::launch::{LaunchDialog, LaunchUpdate};
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

/// One run of the launch flow.
///
/// A run's [`Token`] is invalidated when it is cancelled or superseded, so a
/// late report from a dead run is dropped instead of writing into a fresh one.
pub(crate) struct LaunchController {
    task: Option<tokio::task::JoinHandle<()>>,
    gate: Gate,
}

impl LaunchController {
    fn new() -> Self {
        Self {
            task: None,
            gate: Gate::new(),
        }
    }

    fn begin(&mut self) -> Token {
        self.gate.issue()
    }

    /// Cancels the running flow. Returns whether one was running, which tells
    /// the caller to reload the instance list.
    fn cancel(&mut self) -> bool {
        let had_task = self.task.is_some();
        self.gate.invalidate();
        if let Some(task) = self.task.take() {
            task.abort();
        }
        had_task
    }
}
