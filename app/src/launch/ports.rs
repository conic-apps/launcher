// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch flow's two output ports.
//!
//! [`install_sink`] and [`launch_sink`] are what the app hands the `install` and
//! `launch` crates. Each turns a crate event into a [`presenter::LaunchView`] and
//! delivers it on the event loop, gated by the run's [`Token`] so a cancelled or
//! superseded run cannot draw. The crates own the sampling; all this half does
//! is present.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use slint::Weak;

use super::presenter;
use super::*;

/// The sink `install` reports through. `loader` is the instance's mod loader
/// name, which the loader stages show.
pub(crate) fn install_sink(weak: Weak<App>, token: Token, loader: String) -> install::InstallSink {
    Arc::new(move |progress| {
        let view = presenter::install_view(&progress, &loader);
        deliver(&weak, &token, move |ui| {
            presenter::apply(&ui.global::<LaunchState>(), view);
        });
    })
}

/// The sink `launch` reports through.
pub(crate) fn launch_sink(weak: Weak<App>, token: Token) -> launch::LaunchSink {
    // The game is up on the first of the three startup markers, and the quit
    // dialog has nothing left to warn about from then on. It is read off the
    // raw event (the three still collapse into one screen state), latched so
    // the dismiss runs once.
    let game_up = Arc::new(AtomicBool::new(false));
    Arc::new(move |progress| {
        if matches!(
            progress,
            LaunchProgress::LogLwjglVersion
                | LaunchProgress::LogOpenALLoaded
                | LaunchProgress::LogTextureLoaded
        ) && !game_up.swap(true, Ordering::SeqCst)
        {
            deliver(&weak, &token, |ui| {
                ui.global::<Dialogs>().set_confirm_quit_app_visible(false);
            });
        }
        if let Some(view) = presenter::launch_view(&progress) {
            deliver(&weak, &token, move |ui| {
                presenter::apply(&ui.global::<LaunchState>(), view);
            });
        }
    })
}
