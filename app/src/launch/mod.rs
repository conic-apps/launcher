// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch view's "script" (src/views/LaunchView.vue): it plays the Vue's
//! `launch()` — account refresh, install when needed, then the launch — against
//! the `slint-install` / `slint-launch` crates and pushes progress into the
//! `LaunchState` global.
//!
//! The Vue kept the two tasks as Tauri commands that reported progress over an
//! IPC channel; here the whole flow is one task on the app's runtime, which
//! polls the same `Arc<Mutex<…Event>>` the commands used and translates it into
//! `LaunchState` writes. Cancelling aborts the task, exactly like the Vue's
//! `cancel()` did through `cmd_cancel_*_task`.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use slint::{ComponentHandle, Weak};

use crate::slint_backend::{App, Dialogs, GameState, LaunchState, Navigation};
use account::Account;
use config::Config;
use download::progress::{DownloadPhase, DownloadState};
use install::{InstallEvent, ModLoaderProgress};
use instance::Instance;
use launch::LaunchEvent;

mod events;
mod flow;
mod wiring;

pub(crate) use events::*;
pub(crate) use flow::*;
pub(crate) use wiring::*;

thread_local! {
    /// The app's shared config, for the event-loop half of the account refresh.
    /// The runtime task cannot hold the `Rc<RefCell<…>>` (it is neither `Send`
    /// nor `Sync`); it reports the refreshed account back, and the event-loop
    /// closure writes it through here (and persists it), exactly like the Vue's
    /// `configStore.current_account = …`.
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

/// The controller, for use on the UI thread. Named apart from `setup`'s own
/// `controller` binding, which would otherwise shadow it.
pub(crate) fn launch_controller() -> Rc<RefCell<LaunchController>> {
    CONTROLLER
        .with(|cell| cell.borrow().clone())
        .expect("the launch controller is set up before any of its callbacks run")
}

/// One run of the launch flow. Everything the run schedules carries a clone of
/// it, so a late event from a cancelled run is dropped instead of writing into
/// a fresh one.
#[derive(Clone)]
pub(crate) struct Run {
    id: u64,
    cancelled: Arc<AtomicBool>,
    current_id: Arc<AtomicU64>,
}

impl Run {
    fn is_current(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst) && self.current_id.load(Ordering::SeqCst) == self.id
    }
}

pub(crate) struct LaunchController {
    /// The running flow, aborted by [`LaunchController::cancel`].
    task: Option<tokio::task::JoinHandle<()>>,
    current_id: Arc<AtomicU64>,
    next_id: u64,
    /// The cancelled flags of every run that was started, so `cancel` can stop
    /// their in-flight event deliveries as well as the task itself.
    runs: Vec<Arc<AtomicBool>>,
}

impl LaunchController {
    fn new() -> Self {
        Self {
            task: None,
            current_id: Arc::new(AtomicU64::new(0)),
            next_id: 1,
            runs: Vec::new(),
        }
    }

    fn begin(&mut self) -> Run {
        let id = self.next_id;
        self.next_id += 1;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.runs.push(Arc::clone(&cancelled));
        self.current_id.store(id, Ordering::SeqCst);
        Run {
            id,
            cancelled,
            current_id: Arc::clone(&self.current_id),
        }
    }

    /// Cancels the running flow. Returns whether one was running, which is the
    /// Vue's `onUnmounted` `instanceStore.loadInstances()` trigger.
    fn cancel(&mut self) -> bool {
        let had_task = self.task.is_some();
        for cancelled in self.runs.drain(..) {
            cancelled.store(true, Ordering::SeqCst);
        }
        self.current_id.store(0, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        had_task
    }
}
