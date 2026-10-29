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
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use shared::task::TaskSpec;

use super::*;
use crate::ui::services::tasks::TaskStarter;
use crate::usecases::tasks::TaskHandle;

#[derive(Default)]
struct InstallTask {
    handle: Option<TaskHandle>,
    finished: bool,
}

/// The sink the launch use case reports through.
///
/// The install card is started lazily when the first install event arrives:
/// the launch flow may be refused during its account checks and never touch the
/// disk, and an empty "installing" card would be a lie.
pub(crate) fn update_sink(
    weak: Weak<App>,
    token: Token,
    loader: String,
    starter: TaskStarter,
    install_spec: Option<TaskSpec>,
    task_id: Arc<Mutex<Option<i32>>>,
) -> Sink<LaunchUpdate> {
    // The game is up on the first of the three startup markers, and the quit
    // dialog has nothing left to warn about from then on. Latched so the
    // dismiss runs once.
    let game_up = Arc::new(AtomicBool::new(false));
    let install_task = Mutex::new(InstallTask::default());
    Arc::new(move |update| match update {
        LaunchUpdate::Install(progress) => {
            if let Some(spec) = &install_spec {
                update_install_task(&weak, &starter, &install_task, &task_id, spec, &progress);
            }
            let view = presenter::install_view(&progress, &loader);
            deliver(&weak, &token, move |ui| {
                presenter::apply_view(&ui.global::<LaunchState>(), view);
            });
        }
        LaunchUpdate::Launch(progress) => {
            // The install is over as soon as the launch crate starts its own
            // stages; the launch screen keeps the rest.
            finish_install_task(&install_task, &task_id, None);
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
        LaunchUpdate::Dialog(dialog) => show_dialog(&weak, &token, dialog),
        LaunchUpdate::Error(message) => {
            finish_install_task(&install_task, &task_id, Some(message.clone()));
            set_error(&weak, &token, message);
        }
        LaunchUpdate::Account(account) => store_account(&weak, &token, account),
        LaunchUpdate::Finished {
            quit_app,
            pause_music,
        } => {
            finish_install_task(&install_task, &task_id, None);
            deliver(&weak, &token, move |ui| {
                if pause_music {
                    crate::ui::overlays::music_player::pause();
                }
                if quit_app {
                    let _ = ui.hide();
                    let _ = slint::quit_event_loop();
                } else {
                    ui.global::<Navigation>().invoke_back();
                }
            })
        }
    })
}

fn update_install_task(
    weak: &Weak<App>,
    starter: &TaskStarter,
    state: &Mutex<InstallTask>,
    task_id: &Mutex<Option<i32>>,
    spec: &TaskSpec,
    progress: &install::InstallProgress,
) {
    let mut state = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if state.finished {
        return;
    }
    if state.handle.is_none() {
        let handle = starter.begin(spec.clone());
        let id = handle.id();
        if let Ok(mut slot) = task_id.lock() {
            *slot = Some(id);
        }
        let weak = weak.clone();
        starter.attach_cancel(
            id,
            Arc::new(move || {
                let weak = weak.clone();
                let _ = weak.upgrade_in_event_loop(|ui| {
                    crate::ui::views::launch::launch_controller()
                        .borrow_mut()
                        .cancel();
                    ui.global::<Navigation>().invoke_back();
                });
            }),
        );
        state.handle = Some(handle);
    }
    if let Some(handle) = state.handle.as_ref() {
        let (stage, progress) = crate::usecases::progress::install(progress);
        handle.update(stage, progress);
    }
}

fn finish_install_task(
    state: &Mutex<InstallTask>,
    task_id: &Mutex<Option<i32>>,
    error: Option<String>,
) {
    if let Ok(mut slot) = task_id.lock() {
        *slot = None;
    }
    let mut state = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if state.finished {
        return;
    }
    state.finished = true;
    let Some(handle) = state.handle.take() else {
        return;
    };
    match error {
        Some(error) => handle.fail(error),
        None => handle.succeed(),
    }
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
