// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The cross-surface notification centre.
//!
//! Notifications are kept in Slint models owned by this service so a task can
//! keep reporting after the user navigates away from the page that started it.
//! All functions are called on the event-loop thread.

use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, NotificationItem, NotificationState, NotificationTask};

thread_local! {
    static CONTROLLER: Rc<NotificationController> = Rc::new(NotificationController::new());
}

struct NotificationController {
    notifications: Rc<VecModel<NotificationItem>>,
    tasks: Rc<VecModel<NotificationTask>>,
}

impl NotificationController {
    fn new() -> Self {
        Self {
            notifications: Rc::new(VecModel::default()),
            tasks: Rc::new(VecModel::default()),
        }
    }
}

pub(crate) fn setup(ui: &App) {
    CONTROLLER.with(|controller| {
        let state = ui.global::<NotificationState>();
        state.set_notifications(ModelRc::from(Rc::clone(&controller.notifications)));
        state.set_tasks(ModelRc::from(Rc::clone(&controller.tasks)));
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
            mark_read(&ui);
            ui.global::<NotificationState>().set_visible(false);
        }
    });
    let weak = ui.as_weak();
    state.on_mark_read(move || {
        if let Some(ui) = weak.upgrade() {
            mark_read(&ui);
        }
    });
    let weak = ui.as_weak();
    state.on_open_task(move || {
        if let Some(ui) = weak.upgrade() {
            ui.global::<NotificationState>().set_visible(false);
            ui.global::<crate::slint_backend::Navigation>()
                .invoke_navigate("launch".into());
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

pub(crate) fn start_task(ui: &App, id: &str) {
    CONTROLLER.with(|controller| {
        controller.tasks.set_vec(Vec::new());
        controller.tasks.push(NotificationTask {
            id: id.into(),
            title: SharedString::default(),
            kind: "preparing".into(),
            name: SharedString::default(),
            current: SharedString::default(),
            total: SharedString::default(),
            message: SharedString::default(),
            value: 0.0,
            max: 0.0,
            loading: true,
        });
        log::debug!(target: "notifications", "started task {id}");
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

pub(crate) fn cancel_task(ui: &App) {
    CONTROLLER.with(|controller| {
        controller.tasks.set_vec(Vec::new());
        add_notification(
            controller,
            NotificationItem {
                kind: "task-cancelled".into(),
                detail: SharedString::default(),
                icon: "bell".into(),
            },
        );
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

pub(crate) fn update_task(ui: &App, view: &crate::ui::views::launch::LaunchView) {
    CONTROLLER.with(|controller| {
        let Some(mut task) = controller.tasks.row_data(0) else {
            return;
        };
        task.kind = view.kind().into();
        task.name = view.name().into();
        task.current = view.current().into();
        task.total = view.total().into();
        task.message = view.message().into();
        task.loading = view.loading();
        task.value = view.value();
        task.max = view.max();
        controller.tasks.set_row_data(0, task);
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

pub(crate) fn finish_task(ui: &App, kind: &str, message: &str) {
    CONTROLLER.with(|controller| {
        if controller.tasks.row_count() > 0 {
            controller.tasks.remove(0);
        }
        add_notification(
            controller,
            NotificationItem {
                kind: kind.into(),
                detail: message.into(),
                icon: "bell".into(),
            },
        );
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

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

fn mark_read(ui: &App) {
    CONTROLLER.with(|controller| {
        controller.notifications.set_vec(Vec::new());
        update_count(&ui.global::<NotificationState>(), controller);
    });
}
fn clear(ui: &App) {
    mark_read(ui);
}

fn update_count(state: &NotificationState, controller: &NotificationController) {
    let notification_count = controller.notifications.row_count() as i32;
    let task_count = controller.tasks.row_count() as i32;
    state.set_notification_count(notification_count);
    state.set_task_count(task_count);
    state.set_count(notification_count + task_count);
}

fn add_notification(controller: &NotificationController, item: NotificationItem) {
    controller.notifications.insert(0, item);
}
