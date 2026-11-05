// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Delivering a background result onto the Slint event loop.
//!
//! Slint is not thread-safe and a strong handle cannot cross a thread, so every
//! result arrives through `Weak::upgrade_in_event_loop`; a [`Token`] lets a
//! superseded request be dropped before it draws over a newer one.

use slint::Weak;

use crate::slint_backend::App;
use crate::usecases::generation::Token;

/// Runs `apply` on the event loop if `token` is still current.
pub(crate) fn deliver<F>(weak: &Weak<App>, token: &Token, apply: F)
where
    F: FnOnce(App) + Send + 'static,
{
    let token = token.clone();
    if let Err(error) = weak.upgrade_in_event_loop(move |ui| {
        if token.is_current() {
            apply(ui);
        }
    }) {
        // The event loop is gone, so the result had nowhere to land. Window
        // teardown is the ordinary cause; the other cause looks like a task
        // that silently did nothing, so record it at `debug!` rather than
        // `warn!`.
        log::debug!(target: "shell", "a background result had nowhere to land: {error}");
    }
}

/// Runs `apply` on the event loop with no staleness check.
///
/// For a result that cannot be superseded: a one-shot action whose effect is not
/// a view that a newer request could replace.
pub(crate) fn report<F>(weak: &Weak<App>, apply: F)
where
    F: FnOnce(App) + Send + 'static,
{
    if let Err(error) = weak.upgrade_in_event_loop(apply) {
        log::debug!(target: "shell", "a background result had nowhere to land: {error}");
    }
}
