// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app's running-session fan-out. `launch::running` keeps one subscriber, so
//! the one `subscribe` call lives here and surfaces register as observers.
//!
//! The crate's sink runs on a watcher thread, so every event is delivered onto the
//! event loop before an observer sees it.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use launch::running::{self, SessionEvent};
use shared::Sink;
use slint::ComponentHandle;

use crate::slint_backend::App;
use crate::ui::services::report::report;

thread_local! {
    /// Surfaces that re-read a session after every event. Thread-local because
    /// an `Rc` cannot cross into the `upgrade_in_event_loop` closure that
    /// delivers the event.
    static OBSERVERS: RefCell<Vec<Observer>> = const { RefCell::new(Vec::new()) };
}

type Observer = Rc<dyn Fn(&App, &SessionEvent)>;

/// Registers a callback that runs on the event loop after every session event.
pub(crate) fn observe(observer: impl Fn(&App, &SessionEvent) + 'static) {
    OBSERVERS.with(|observers| observers.borrow_mut().push(Rc::new(observer)));
}

/// Installs the app's one session subscriber. Runs once, before any launch.
pub(crate) fn setup(ui: &App) {
    let weak = ui.as_weak();
    let sink: Sink<SessionEvent> = Arc::new(move |event| {
        let weak = weak.clone();
        report(&weak, move |ui| notify(&ui, &event));
    });
    let running = running::subscribe(sink);
    log::debug!(
        "subscribed to the launch session events; {} session(s) already running",
        running.len()
    );
}

fn notify(ui: &App, event: &SessionEvent) {
    OBSERVERS.with(|observers| {
        for observer in observers.borrow().iter() {
            observer(ui, event);
        }
    });
}
