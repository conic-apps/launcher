// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch page's callbacks and the app's one way to start a launch.
//!
//! [`request`] is the entry point every surface uses — the game view's launch
//! button, the notice behind a finished task, the command palette. The page opens
//! at once and the flow starts when the task centre goes idle, so the wait has
//! somewhere to be shown and a cancel button.

use super::*;

pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    SHARED_CONFIG.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&config)));
    let controller = Rc::new(RefCell::new(LaunchController::new()));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&controller)));

    {
        let weak = ui.as_weak();
        ui.global::<LaunchState>().on_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                // Leaving the page is the cancel: [`page_left`] owns the
                // teardown, so the back button and any other way off the page
                // cannot disagree.
                ui.global::<Navigation>().invoke_back();
            }
        });
    }

    // The teardown of a page that was left is driven by `main`'s single
    // `Navigation.page-changed` handler, which calls [`page_left`]: a Slint
    // callback keeps one handler, so a second `on_page_changed` here would be
    // replaced by that one later in `main` without a word.

    // A waiting launch is retried on every task event: the centre going idle is
    // what releases it, and a task ending is the only thing that can do that.
    crate::ui::services::tasks::observe(try_start);
}

/// The launch page was left: the waiting request or the running flow ends with it,
/// and a process the flow had already spawned is stopped with it. Called from
/// `main`'s single `Navigation.page-changed` handler; without it a launch left
/// behind keeps its request pending and blocks every later one.
pub(crate) fn page_left(ui: &App, page: &str) {
    if page == "launch" {
        return;
    }
    let controller = launch_controller();
    if !controller.borrow_mut().cancel() {
        return;
    }
    log::info!(target: "launch", "the launch page was left; the launch was cancelled");
    // The instance list is reloaded, so what the launch left on disk (a play
    // time) shows up at once.
    ui.global::<GameState>().invoke_refresh();
}

/// Back to the game view, whether or not there is a page to go back to.
///
/// The launch page is only ever entered from the game view, so a `previous-page`
/// of "" means an earlier `back()` already consumed it — navigating there again
/// is the same destination and cannot leave the player on a dead screen.
pub(crate) fn return_to_game_view(ui: &App) {
    let navigation = ui.global::<Navigation>();
    if navigation.get_previous_page().is_empty() {
        navigation.invoke_navigate("game".into());
    } else {
        navigation.invoke_back();
    }
    log::debug!(
        target: "launch",
        "the launch is over; the page is now '{}'",
        navigation.get_current_page()
    );
}

/// Asks for a launch of `instance_id`.
///
/// Idempotent: a second ask while one launch is waiting or running is dropped.
pub(crate) fn request(ui: &App, instance_id: String) {
    if launch_controller().borrow().busy() {
        log::warn!(
            target: "launch",
            "a launch of '{instance_id}' was ignored: another one is waiting or running"
        );
        return;
    }
    log::info!(target: "launch", "launch requested for '{instance_id}'");
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        // `instance.toml` is read on the runtime; everything below runs on the
        // event loop, where the centre and Slint live.
        let Some(instance) = instance::get_instance_by_id(&instance_id).await else {
            log::error!("failed to read instance '{instance_id}' for the launch");
            return;
        };
        crate::ui::services::report::report(&weak, move |ui| enter(&ui, instance));
    });
}

/// Opens the launch page on the instance and either starts the flow or waits.
fn enter(ui: &App, instance: Instance) {
    if launch_controller().borrow().busy() {
        log::warn!(
            target: "launch",
            "the launch of '{}' was dropped: another one started first",
            instance.config.name
        );
        return;
    }
    if !instance.installed {
        // Nothing offers a launch for an uninstalled instance, but the command
        // palette and the notices can reach this path; an install is what the
        // player is after, and it queues behind whatever holds the lane.
        crate::ui::views::game::install::start(ui, instance.id);
        return;
    }
    let config = SHARED_CONFIG.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|config| config.borrow().clone())
            .unwrap_or_default()
    });
    // Seeded before the navigation, so the page's intro sees an instance on its
    // first frame instead of waiting for the flow's first report.
    reset_state(ui, Some(&instance), &config);
    show_waiting(ui);
    launch_controller().borrow_mut().wait(instance, config);
    ui.global::<Navigation>().invoke_navigate("launch".into());
    try_start(ui);
}

/// Starts the flow when the centre has nothing left to do.
fn try_start(ui: &App) {
    let controller = launch_controller();
    let ready = controller.borrow().pending.is_some() && !controller.borrow().task.is_some();
    if !ready {
        return;
    }
    let Some(reservation) = crate::ui::services::tasks::service().reserve_exclusive() else {
        log::debug!(
            target: "launch",
            "the task centre is busy; the launch keeps waiting"
        );
        return;
    };
    let Some((instance, config)) = controller.borrow_mut().take_pending() else {
        return;
    };
    start_flow(ui, instance, config, reservation);
}

/// The flow proper: the page stops saying it is waiting and the launch begins.
fn start_flow(ui: &App, instance: Instance, config: Config, reservation: TaskReservation) {
    log::info!(
        target: "launch",
        "the task centre is idle; starting the launch of '{}'",
        instance.config.name
    );
    let controller = launch_controller();
    let token = controller.borrow_mut().begin();
    // The first stage report replaces the waiting line; until then the player
    // sees this instead of an empty bar.
    ui.global::<LaunchState>()
        .set_progress_kind("preparing-launch".into());
    ui.global::<LaunchState>().set_progress_loading(true);

    let sink = update_sink(ui.as_weak(), token);
    let handle = crate::support::runtime::spawn(async move {
        // Re-read: the request may have waited in the queue for the install that
        // writes `.install.lock`, and the copy read at request time predates it.
        let instance = instance::get_instance_by_id(&instance.id)
            .await
            .unwrap_or(instance);
        crate::usecases::launch::run(config, Some(instance), sink).await;
    });
    controller.borrow_mut().running(handle, reservation);
}

/// The waiting line, shown for as long as the centre is busy.
fn show_waiting(ui: &App) {
    let state = ui.global::<LaunchState>();
    state.set_error(false);
    state.set_progress_kind("waiting-tasks".into());
    state.set_progress_loading(true);
    state.set_progress_value(0.0);
    state.set_progress_max(0.0);
}
