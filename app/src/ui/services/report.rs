// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Delivering a background result onto the Slint event loop.
//!
//! Everything a background task produces has to reach the UI through
//! `Weak::upgrade_in_event_loop`: Slint is not thread-safe, and the strong
//! handle cannot cross a thread. The two helpers here are the one way the app
//! does that, so a task reports through a [`Gate`]'s [`Token`] rather than
//! open-coding the same marshalling, and a superseded request cannot draw over
//! a newer one.
//!
//! A task that produces a *stream* of progress reports through a crate's
//! `Sink` instead, and its sink implementation calls [`deliver`] here. The two
//! live together: the crate owns the port, this module owns the hop onto the
//! event loop.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use slint::Weak;

use crate::slint_backend::App;

/// One logical request's claim on the interface.
///
/// A token is issued by a [`Gate`] and checked on the event loop before its
/// result is applied. When the gate issues a newer token — or is invalidated —
/// every older token stops being current, so a slow answer that comes back
/// after a newer request cannot overwrite it.
#[derive(Clone)]
pub(crate) struct Token {
    generation: Arc<AtomicU64>,
    mine: u64,
}

impl Token {
    /// Whether this request is still the newest one.
    pub(crate) fn is_current(&self) -> bool {
        self.generation.load(Ordering::SeqCst) == self.mine
    }
}

/// Issues [`Token`]s for one logical slot.
///
/// One gate per slot — a list, a detail panel, a launch flow — so a newer
/// request in one slot does not cancel an unrelated one elsewhere.
#[derive(Clone, Default)]
pub(crate) struct Gate {
    generation: Arc<AtomicU64>,
}

impl Gate {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Starts a new request, invalidating every earlier token.
    pub(crate) fn issue(&self) -> Token {
        let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        Token {
            generation: Arc::clone(&self.generation),
            mine,
        }
    }

    /// Invalidates the outstanding token, so nothing in flight is delivered.
    pub(crate) fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// Runs `apply` on the event loop if `token` is still current.
///
/// `apply` receives the component by value, exactly as
/// `Weak::upgrade_in_event_loop` hands it over, so a caller can keep using the
/// same body it would have written for that call.
pub(crate) fn deliver<F>(weak: &Weak<App>, token: &Token, apply: F)
where
    F: FnOnce(App) + Send + 'static,
{
    let token = token.clone();
    let _ = weak.upgrade_in_event_loop(move |ui| {
        if token.is_current() {
            apply(ui);
        }
    });
}

/// Runs `apply` on the event loop with no staleness check.
///
/// For a result that cannot be superseded: a one-shot action whose effect is not
/// a view that a newer request could replace.
pub(crate) fn report<F>(weak: &Weak<App>, apply: F)
where
    F: FnOnce(App) + Send + 'static,
{
    let _ = weak.upgrade_in_event_loop(apply);
}
