// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Delivering a background result onto the Slint event loop.
//!
//! Everything a background task produces has to reach the UI through
//! `Weak::upgrade_in_event_loop`: Slint is not thread-safe, and the strong
//! handle cannot cross a thread. The two helpers here are the one way the app
//! does that, so a task reports through a [`Token`]
//! rather than open-coding the same marshalling, and a superseded request
//! cannot draw over a newer one.
//!
//! The token itself lives in `usecases::generation` — it is a use-case concern.
//! A task that produces a *stream* of progress reports through a crate's `Sink`
//! instead, and its sink implementation calls [`deliver`] here.

use slint::Weak;

use crate::slint_backend::App;
use crate::usecases::generation::Token;

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
