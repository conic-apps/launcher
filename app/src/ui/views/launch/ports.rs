// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch flow's output port.
//!
//! [`update_sink`] is what the app hands [`crate::usecases::launch::run`]. It
//! turns each [`LaunchUpdate`] into `LaunchState` writes (through
//! [`presenter`]) and the one-off dialogs, gated by the run's [`Token`] so a
//! cancelled or superseded run cannot draw.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::*;

/// The sink the launch use case reports through.
pub(crate) fn update_sink(weak: Weak<App>, token: Token, loader: String) -> Sink<LaunchUpdate> {
    // The game is up on the first of the three startup markers, and the quit
    // dialog has nothing left to warn about from then on. Latched so the
    // dismiss runs once.
    let game_up = Arc::new(AtomicBool::new(false));
    Arc::new(move |update| match update {
        LaunchUpdate::Install(progress) => {
            let view = presenter::install_view(&progress, &loader);
            deliver(&weak, &token, move |ui| {
                presenter::apply(&ui.global::<LaunchState>(), view);
            });
        }
        LaunchUpdate::Launch(progress) => {
            if matches!(
                progress,
                launch::LaunchProgress::LogLwjglVersion
                    | launch::LaunchProgress::LogOpenALLoaded
                    | launch::LaunchProgress::LogTextureLoaded
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
        }
        LaunchUpdate::RefreshAccount => push(&weak, &token, |state| {
            state.set_progress_kind("refresh-account".into());
            state.set_progress_loading(true);
        }),
        LaunchUpdate::Dialog(dialog) => show_dialog(&weak, &token, dialog),
        LaunchUpdate::Error(message) => set_error(&weak, &token, message),
        LaunchUpdate::Account(account) => store_account(&weak, &token, account),
        LaunchUpdate::Finished {
            quit_app,
            pause_music,
        } => deliver(&weak, &token, move |ui| {
            if pause_music {
                crate::ui::overlays::music_player::pause();
            }
            if quit_app {
                let _ = ui.hide();
                let _ = slint::quit_event_loop();
            } else {
                ui.global::<Navigation>().invoke_back();
            }
        }),
    })
}

/// Shows a refusal or failure dialog.
fn show_dialog(weak: &Weak<App>, token: &Token, dialog: LaunchDialog) {
    deliver(weak, token, move |ui| {
        let dialogs = ui.global::<Dialogs>();
        match dialog {
            LaunchDialog::NoAccount => dialogs.set_no_account_error_visible(true),
            LaunchDialog::NoMicrosoftAccount => {
                dialogs.set_no_microsoft_account_error_visible(true)
            }
            LaunchDialog::AccountRefreshFailed => dialogs.set_account_refresh_failed_visible(true),
            LaunchDialog::NoSuitableJava => dialogs.set_no_suitable_java_error_visible(true),
        }
    });
}

/// Shows an error message on the launch screen.
fn set_error(weak: &Weak<App>, token: &Token, message: String) {
    deliver(weak, token, move |ui| {
        let state = ui.global::<LaunchState>();
        state.set_error(true);
        state.set_progress_kind("error".into());
        state.set_error_message(message.into());
        state.set_back_disabled(false);
    });
}

/// Pushes the refreshed account into the shared config and the view, on the
/// event loop.
fn store_account(weak: &Weak<App>, token: &Token, account: Account) {
    deliver(weak, token, move |ui| {
        SHARED_CONFIG.with(|slot| {
            if let Some(config) = slot.borrow().as_ref() {
                config.borrow_mut().current_account = Some(account.clone());
                if let Err(error) = config::save_config(&config.borrow()) {
                    log::warn!("failed to save the refreshed account: {error}");
                }
            }
        });
        let state = ui.global::<LaunchState>();
        state.set_has_account(true);
        state.set_account_kind(account.kind().into());
        state.set_profile_name(account.get_profile_name().into());
        state.set_account_avatar(
            crate::ui::components::account_avatar::account_head(Some(&account), 48)
                .unwrap_or_default(),
        );
    });
}

/// Pushes one `LaunchState` write on the event loop, dropping it if the run has
/// been superseded.
fn push(weak: &Weak<App>, token: &Token, update: impl FnOnce(&LaunchState) + Send + 'static) {
    deliver(weak, token, move |ui| {
        update(&ui.global::<LaunchState>());
    });
}
