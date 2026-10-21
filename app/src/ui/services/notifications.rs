// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The cross-surface notification centre.
//!
//! Notifications are kept in Slint models owned by this service so a producer
//! can keep reporting after the user navigates away from the page that started
//! it. All functions are called on the event-loop thread.

use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::slint_backend::{App, NotificationItem, NotificationState};

thread_local! {
    static CONTROLLER: Rc<NotificationController> = Rc::new(NotificationController::new());
}

struct NotificationController {
    notifications: Rc<VecModel<NotificationItem>>,
}

impl NotificationController {
    fn new() -> Self {
        Self {
            notifications: Rc::new(VecModel::default()),
        }
    }
}

pub(crate) fn setup(ui: &App) {
    CONTROLLER.with(|controller| {
        let state = ui.global::<NotificationState>();
        state.set_notifications(ModelRc::from(Rc::clone(&controller.notifications)));
        update_count(&state, controller);
    });

    let weak = ui.as_weak();
    ui.on_open_notifications(move || {
        if let Some(ui) = weak.upgrade() {
            toggle(&ui);
        }
    });

    let state = ui.global::<NotificationState>();
    let weak = ui.as_weak();
    state.on_close(move || {
        if let Some(ui) = weak.upgrade() {
            ui.global::<NotificationState>().set_visible(false);
        }
    });
    let weak = ui.as_weak();
    state.on_clear(move || {
        if let Some(ui) = weak.upgrade() {
            clear(&ui);
        }
    });
}

pub(crate) fn toggle(ui: &App) {
    let state = ui.global::<NotificationState>();
    state.set_visible(!state.get_visible());
}

/// Temporary UI fixture: fills the centre with one of each row style shortly
/// after startup, so the panel can be inspected without a real producer.
#[cfg(debug_assertions)]
pub(crate) fn schedule_demo(ui: &App) {
    log::debug!(target: "notifications", "scheduling demo notifications");
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        crate::ui::services::report::report(&weak, |ui| {
            log::debug!(target: "notifications", "sending demo notifications");
            push(&ui, "update-ready", "0.1.0-alpha.5", "download");
            push(
                &ui,
                "task-finished",
                "Demo task finished",
                "checkmark-outline",
            );
            push(
                &ui,
                "task-error",
                "Demo task failed: the download could not be completed.",
                "warning",
            );
            push(&ui, "task-cancelled", "", "ban");
            push(&ui, "update-ready", "0.1.0-alpha.4", "download");
            push(
                &ui,
                "task-finished",
                "Demo task finished",
                "checkmark-outline",
            );
            push(
                &ui,
                "task-error",
                "Demo task failed: another download could not be completed.",
                "warning",
            );
            push(&ui, "task-cancelled", "", "ban");
            push(
                &ui,
                "task-finished",
                "Demo task finished",
                "checkmark-outline",
            );
            push(&ui, "update-ready", "0.1.0-alpha.3", "download");
            push(
                &ui,
                "task-error",
                "Demo task failed: the instance could not be launched.",
                "warning",
            );
        });
    });
}

// No producer reports yet; the centre is mounted ahead of its first source.
#[allow(dead_code)]
pub(crate) fn push(ui: &App, kind: &str, detail: &str, icon: &str) {
    CONTROLLER.with(|controller| {
        add_notification(
            controller,
            NotificationItem {
                kind: kind.into(),
                detail: detail.into(),
                icon: icon.into(),
            },
        );
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

fn clear(ui: &App) {
    CONTROLLER.with(|controller| {
        controller.notifications.set_vec(Vec::new());
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

fn update_count(state: &NotificationState, controller: &NotificationController) {
    let count = controller.notifications.row_count() as i32;
    state.set_notification_count(count);
}

fn add_notification(controller: &NotificationController, item: NotificationItem) {
    controller.notifications.insert(0, item);
}
