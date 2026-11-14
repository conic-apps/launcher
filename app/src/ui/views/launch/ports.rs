// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch flow's output port: [`update_sink`] turns each [`LaunchUpdate`] into
//! `LaunchState` writes and one-off dialogs, gated by the run's [`Token`]. A
//! terminal report settles the controller and hands the exclusive lane back.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::ui::services::report::report;

use super::*;

/// The sink the launch use case reports through.
pub(crate) fn update_sink(weak: Weak<App>, token: Token) -> Sink<LaunchUpdate> {
    // The game is up on the first of the three startup markers, and the quit
    // dialog has nothing left to warn about from then on. Latched so the
    // dismiss runs once.
    let game_up = Arc::new(AtomicBool::new(false));
    Arc::new(move |update| match update {
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
                    presenter::apply_view(&ui.global::<LaunchState>(), view);
                });
            }
        }
        LaunchUpdate::RefreshAccount => push(&weak, &token, |state| {
            state.set_progress_kind("refresh-account".into());
            state.set_progress_loading(true);
        }),
        LaunchUpdate::Dialog(dialog) => {
            // The dialog is the whole answer; the page stays as it is, and the
            // lane comes back so a queued install can run while the player reads
            // it.
            log::info!(target: "launch", "the launch was refused after the flow started");
            settle_on_event_loop(&weak);
            show_dialog(&weak, &token, dialog);
        }
        LaunchUpdate::Error(message) => {
            settle_on_event_loop(&weak);
            set_error(&weak, &token, message);
        }
        LaunchUpdate::Account(account) => store_account(&weak, &token, account),
        LaunchUpdate::Finished {
            quit_app,
            pause_music,
        } => {
            settle_on_event_loop(&weak);
            deliver(&weak, &token, move |ui| {
                if pause_music {
                    crate::ui::overlays::music_player::pause();
                }
                if quit_app {
                    let _ = ui.hide();
                    let _ = slint::quit_event_loop();
                } else {
                    // The launch is what moved the play time on disk.
                    ui.global::<GameState>().invoke_refresh();
                    return_to_game_view(&ui);
                }
            })
        }
    })
}

/// Hands the exclusive lane back, on the event loop. The controller is a
/// thread-local and the sink runs on a worker thread, so this cannot be called from
/// the sink itself. Ungated on purpose — the lane must come back even when the token
/// is stale, or a launch the player abandoned would hold it.
fn settle_on_event_loop(weak: &Weak<App>) {
    report(weak, |_| launch_controller().borrow_mut().settle());
}

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

fn set_error(weak: &Weak<App>, token: &Token, message: String) {
    deliver(weak, token, move |ui| {
        let state = ui.global::<LaunchState>();
        state.set_error(true);
        state.set_progress_kind("error".into());
        state.set_error_message(message.into());
    });
}

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

fn push(weak: &Weak<App>, token: &Token, update: impl FnOnce(&LaunchState) + Send + 'static) {
    deliver(weak, token, move |ui| {
        update(&ui.global::<LaunchState>());
    });
}
