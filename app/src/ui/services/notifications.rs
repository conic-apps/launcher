// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The cross-surface notification centre.
//!
//! Notifications are kept in Slint models owned by this service so a producer
//! can keep reporting after the user navigates away from the page that started
//! it. A Slint model only carries plain data, so the card's action cannot live
//! in it: the controller keeps an `id -> action` table beside the model and the
//! view calls back with that id. All functions except the [`NoticeSink`] are
//! called on the event-loop thread; the sink marshals from any thread.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, NotificationItem, NotificationState};
use crate::ui::services::report;
use shared::notification::{Notice, NoticeAction, NoticeKind, NoticeSink};

thread_local! {
    static CONTROLLER: Rc<NotificationController> = Rc::new(NotificationController::new());
}

struct PendingAction {
    action: NoticeAction,
    dismiss: bool,
}

struct NotificationController {
    notifications: Rc<VecModel<NotificationItem>>,
    actions: RefCell<HashMap<i32, PendingAction>>,
    next_id: Cell<i32>,
}

impl NotificationController {
    fn new() -> Self {
        Self {
            notifications: Rc::new(VecModel::default()),
            actions: RefCell::new(HashMap::new()),
            next_id: Cell::new(0),
        }
    }

    fn send(&self, notice: Notice) {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));
        let has_action = notice.action.is_some();
        let dismiss = notice.dismiss_on_click;
        let item = NotificationItem {
            id,
            kind: kind_key(notice.kind).into(),
            body: notice.body.unwrap_or_default().into(),
            args: ModelRc::from(Rc::new(VecModel::from(
                notice
                    .args
                    .into_iter()
                    .map(SharedString::from)
                    .collect::<Vec<_>>(),
            ))),
            icon: notice
                .icon
                .unwrap_or_else(|| kind_icon(notice.kind).to_owned())
                .into(),
            has_action,
        };
        self.notifications.insert(0, item);
        if let Some(action) = notice.action {
            self.actions
                .borrow_mut()
                .insert(id, PendingAction { action, dismiss });
        }
    }

    /// Takes the card's action without removing the card.
    ///
    /// The caller runs it first and removes the card afterwards, so the click
    /// keeps the card visible while the action is working. Taking the action
    /// separately also makes a re-entrant click a no-op instead of running it
    /// twice.
    fn take_action(&self, id: i32) -> Option<PendingAction> {
        let actionable = self
            .notifications
            .iter()
            .any(|item| item.id == id && item.has_action);
        if !actionable {
            return None;
        }
        self.actions.borrow_mut().remove(&id)
    }

    fn remove(&self, id: i32) -> bool {
        let Some(index) = self.notifications.iter().position(|item| item.id == id) else {
            return false;
        };
        self.actions.borrow_mut().remove(&id);
        self.notifications.remove(index);
        true
    }

    /// Disables a card that stays in the centre after its action ran.
    fn mark_spent(&self, id: i32) -> bool {
        let Some(index) = self.notifications.iter().position(|item| item.id == id) else {
            return false;
        };
        let Some(mut item) = self.notifications.row_data(index) else {
            return false;
        };
        if !item.has_action {
            return false;
        }
        item.has_action = false;
        self.notifications.set_row_data(index, item);
        true
    }

    fn clear(&self) {
        self.notifications.set_vec(Vec::new());
        self.actions.borrow_mut().clear();
    }
}

/// Registers the Slint callbacks and returns the port a producer posts through.
pub(crate) fn setup(ui: &App) -> NoticeSink {
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
    let weak = ui.as_weak();
    state.on_dismiss(move |id| {
        if let Some(ui) = weak.upgrade() {
            dismiss(&ui, id);
        }
    });
    let weak = ui.as_weak();
    state.on_activate(move |id| {
        if let Some(ui) = weak.upgrade() {
            activate(&ui, id);
        }
    });

    sink(ui)
}

pub(crate) fn toggle(ui: &App) {
    let state = ui.global::<NotificationState>();
    state.set_visible(!state.get_visible());
}

/// Posts a notice from the event-loop thread.
pub(crate) fn send(ui: &App, notice: Notice) {
    log::debug!(
        target: "notifications",
        "posting {:?} notice ({} args)",
        notice.kind,
        notice.args.len()
    );
    CONTROLLER.with(|controller| {
        controller.send(notice);
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

fn sink(ui: &App) -> NoticeSink {
    let weak = ui.as_weak();
    Arc::new(move |notice| {
        report::report(&weak, move |ui| send(&ui, notice));
    })
}

fn activate(ui: &App, id: i32) {
    let pending = CONTROLLER.with(|controller| controller.take_action(id));
    let Some(pending) = pending else {
        return;
    };
    // Run the caller's action first, then apply the notice's own close policy.
    // The action may itself clear or dismiss notices, so both paths are
    // idempotent by id.
    (pending.action)();
    CONTROLLER.with(|controller| {
        if pending.dismiss {
            controller.remove(id);
        } else {
            controller.mark_spent(id);
        }
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

fn dismiss(ui: &App, id: i32) {
    CONTROLLER.with(|controller| {
        controller.remove(id);
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

fn clear(ui: &App) {
    CONTROLLER.with(|controller| {
        controller.clear();
        update_count(&ui.global::<NotificationState>(), controller);
    });
}

fn update_count(state: &NotificationState, controller: &NotificationController) {
    let count = controller.notifications.row_count() as i32;
    state.set_notification_count(count);
}

fn kind_key(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::UpdateReady => "update-ready",
        NoticeKind::TaskFinished => "task-finished",
        NoticeKind::TaskFailed => "task-error",
        NoticeKind::TaskCancelled => "task-cancelled",
        NoticeKind::SetupSkipped => "setup-skipped",
        NoticeKind::Custom => "custom",
    }
}

fn kind_icon(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::UpdateReady => "download",
        NoticeKind::TaskFinished => "checkmark-outline",
        NoticeKind::TaskFailed => "warning",
        NoticeKind::TaskCancelled => "ban",
        NoticeKind::SetupSkipped => "refresh",
        NoticeKind::Custom => "bell",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use slint::Model;

    use super::*;

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn ids_are_stable_and_an_action_runs_once() {
        let controller = NotificationController::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hit = Arc::clone(&hits);
        controller.send(Notice::new(NoticeKind::TaskFinished).on_click(move || {
            hit.fetch_add(1, Ordering::SeqCst);
        }));

        let item = controller.notifications.row_data(0).expect("one notice");
        assert!(item.id >= 0);
        assert!(item.has_action);
        assert_eq!(item.kind.as_str(), "task-finished");

        let pending = controller.take_action(item.id).expect("an action");
        assert!(pending.dismiss);
        assert_eq!(controller.notifications.row_count(), 1);
        assert!(controller.actions.borrow().is_empty());
        (pending.action)();
        assert!(controller.remove(item.id));
        assert_eq!(controller.notifications.row_count(), 0);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(controller.take_action(item.id).is_none());
    }

    #[test]
    fn a_kept_card_loses_its_action_but_stays_in_the_centre() {
        let controller = NotificationController::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hit = Arc::clone(&hits);
        controller.send(
            Notice::new(NoticeKind::TaskFinished)
                .keep_open_on_click()
                .on_click(move || {
                    hit.fetch_add(1, Ordering::SeqCst);
                }),
        );

        let item = controller.notifications.row_data(0).expect("one notice");
        let pending = controller.take_action(item.id).expect("an action");
        assert!(!pending.dismiss);
        (pending.action)();
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        assert!(controller.mark_spent(item.id));
        let item = controller.notifications.row_data(0).expect("kept notice");
        assert!(!item.has_action);
        assert_eq!(controller.notifications.row_count(), 1);
        assert!(controller.take_action(item.id).is_none());
    }

    #[test]
    fn a_plain_notice_is_not_actionable() {
        let controller = NotificationController::new();
        controller.send(Notice::new(NoticeKind::TaskCancelled));

        let item = controller.notifications.row_data(0).expect("one notice");
        assert!(!item.has_action);
        assert!(controller.take_action(item.id).is_none());
        assert_eq!(controller.notifications.row_count(), 1);
    }

    #[test]
    fn args_reach_the_view_model_in_order() {
        let controller = NotificationController::new();
        controller.send(
            Notice::new(NoticeKind::UpdateReady)
                .with_arg("0.1.0")
                .with_arg("beta"),
        );

        let item = controller.notifications.row_data(0).expect("one notice");
        assert_eq!(item.args.row_count(), 2);
        assert_eq!(
            item.args.row_data(0).map(|value| value.to_string()),
            Some("0.1.0".to_owned())
        );
        assert_eq!(
            item.args.row_data(1).map(|value| value.to_string()),
            Some("beta".to_owned())
        );
    }

    #[test]
    fn a_custom_body_reaches_the_view_model() {
        let controller = NotificationController::new();
        controller.send(Notice::custom("Custom body"));

        let item = controller.notifications.row_data(0).expect("one notice");
        assert_eq!(item.kind.as_str(), "custom");
        assert_eq!(item.body.as_str(), "Custom body");
        assert_eq!(item.icon.as_str(), "bell");
    }

    #[test]
    fn clearing_drops_unrun_actions() {
        let controller = NotificationController::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = DropFlag(Arc::clone(&dropped));
        controller.send(Notice::new(NoticeKind::TaskFailed).on_click(move || {
            drop(flag);
        }));

        controller.clear();
        assert!(dropped.load(Ordering::SeqCst));
        assert_eq!(controller.notifications.row_count(), 0);
        assert!(controller.actions.borrow().is_empty());
    }

    #[test]
    fn dismissing_removes_by_id_and_drops_its_action() {
        let controller = NotificationController::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = DropFlag(Arc::clone(&dropped));
        controller.send(Notice::new(NoticeKind::TaskFailed).on_click(move || {
            drop(flag);
        }));
        let item = controller.notifications.row_data(0).expect("one notice");

        assert!(controller.remove(item.id));
        assert!(dropped.load(Ordering::SeqCst));
        assert!(!controller.remove(item.id));
        assert_eq!(controller.notifications.row_count(), 0);
    }
}
