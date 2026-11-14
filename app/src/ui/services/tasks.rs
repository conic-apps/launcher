// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The task centre's Slint adapter: it projects [`TaskCenter`]'s events into
//! `TaskState`, applies the card's click targets and drives the confirmation
//! dialog. Everything here runs on the event loop.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

use shared::{
    notification::{NoticeAction, NoticeSink},
    task::{TaskEvent, TaskKind, TaskProgress, TaskSpec, TaskTarget},
};

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};

use crate::slint_backend::{
    App, ContentState, Dialogs, GameState, MultiplayerState, Navigation, NotificationState,
    TaskItem, TaskState,
};
use crate::support::formatting::bytes;
use crate::ui::services::report;
use crate::usecases::tasks::{CancelDecision, TaskCenter, TaskNoticeAction};

thread_local! {
    static CONTROLLER: Rc<TaskController> = Rc::new(TaskController::new());
    static SERVICE: RefCell<Option<Rc<TaskService>>> = const { RefCell::new(None) };
    /// Surfaces that want to know when the registry changed, e.g. the game view's
    /// launch button, which mirrors one instance's task into its own state.
    static OBSERVERS: RefCell<Vec<Observer>> = const { RefCell::new(Vec::new()) };
}

/// A surface that re-reads the registry after every task event.
type Observer = Rc<dyn Fn(&App)>;

/// Registers a callback that runs after every task event was applied.
///
/// The service knows nothing about the surfaces that observe it, and the
/// observer sees the registry through [`TaskService::active_on`] rather than a
/// second copy of the state.
pub(crate) fn observe(observer: impl Fn(&App) + 'static) {
    OBSERVERS.with(|observers| observers.borrow_mut().push(Rc::new(observer)));
}

fn notify(ui: &App) {
    OBSERVERS.with(|observers| {
        for observer in observers.borrow().iter() {
            observer(ui);
        }
    });
}

struct TaskController {
    tasks: Rc<VecModel<TaskItem>>,
    toasts: Rc<VecModel<TaskItem>>,
    targets: RefCell<HashMap<i32, TaskTarget>>,
    pending_cancel: Cell<i32>,
}

impl TaskController {
    fn new() -> Self {
        Self {
            tasks: Rc::new(VecModel::default()),
            toasts: Rc::new(VecModel::default()),
            targets: RefCell::new(HashMap::new()),
            pending_cancel: Cell::new(-1),
        }
    }

    fn index_of(&self, id: i32) -> Option<usize> {
        self.tasks.iter().position(|item| item.id == id)
    }

    fn update_toast(&self, id: i32, row: TaskItem) {
        if let Some(index) = self.toasts.iter().position(|item| item.id == id) {
            self.toasts.set_row_data(index, row);
        }
    }

    fn remove_toast(&self, id: i32) {
        if let Some(index) = self.toasts.iter().position(|item| item.id == id) {
            self.toasts.remove(index);
        }
    }

    fn close_toast(&self, id: i32) {
        let Some(index) = self.toasts.iter().position(|item| item.id == id) else {
            return;
        };
        let Some(mut row) = self.toasts.row_data(index) else {
            return;
        };
        if !row.closing {
            row.closing = true;
            self.toasts.set_row_data(index, row);
        }
    }

    fn set_pending(&self, id: i32) {
        self.pending_cancel.set(id);
    }

    fn take_pending(&self) -> i32 {
        self.pending_cancel.replace(-1)
    }
}

/// The handle the app's surfaces start tasks through.
pub(crate) struct TaskService {
    center: Arc<TaskCenter>,
}

impl TaskService {
    /// Hands a task to the scheduler; `work` runs when the centre admits it.
    pub(crate) fn submit(
        &self,
        spec: TaskSpec,
        work: crate::usecases::tasks::TaskWork,
    ) -> shared::task::TaskId {
        self.center.submit(spec, work)
    }

    /// The oldest active task of `kind` for `instance_id`, if any.
    pub(crate) fn active_on(
        &self,
        kind: TaskKind,
        instance_id: &str,
    ) -> Option<shared::task::TaskSnapshot> {
        self.center.active_on(kind, instance_id)
    }

    pub(crate) fn attach_cancel(&self, id: i32, cancel: shared::task::TaskCancel) {
        self.center.attach_cancel(id, cancel);
    }

    /// Stops a task without the elapsed-time prompt. For a superseded task that
    /// is being replaced by a new one, where a dialog would be noise.
    pub(crate) fn cancel_now(&self, id: i32) {
        self.center.cancel_now(id);
    }

    /// Takes the exclusive lane for work the centre does not schedule — the
    /// launch. `None` while any task is still active.
    pub(crate) fn reserve_exclusive(&self) -> Option<crate::usecases::tasks::TaskReservation> {
        self.center.reserve_exclusive()
    }

    fn request_cancel(&self, ui: &App, id: i32) {
        match self.center.request_cancel(id) {
            CancelDecision::Immediate | CancelDecision::Unknown => {}
            CancelDecision::NeedsConfirm(snapshot) => {
                CONTROLLER.with(|controller| controller.set_pending(id));
                ui.global::<TaskState>()
                    .set_pending_cancel_subject(SharedString::from(snapshot.spec.subject));
                ui.global::<Dialogs>().set_confirm_cancel_task_visible(true);
            }
        }
    }

    fn confirm_cancel(&self, ui: &App) {
        let id = CONTROLLER.with(|controller| controller.take_pending());
        if id >= 0 {
            self.center.cancel_now(id);
        }
        close_confirmation(ui);
    }

    fn dismiss_cancel(&self, ui: &App) {
        CONTROLLER.with(|controller| controller.set_pending(-1));
        close_confirmation(ui);
    }
}

fn close_confirmation(ui: &App) {
    ui.global::<TaskState>()
        .set_pending_cancel_subject(SharedString::default());
    ui.global::<Dialogs>()
        .set_confirm_cancel_task_visible(false);
}

pub(crate) fn setup(ui: &App, notices: NoticeSink) -> Rc<TaskService> {
    CONTROLLER.with(|controller| {
        ui.global::<TaskState>()
            .set_tasks(ModelRc::from(Rc::clone(&controller.tasks)));
        ui.global::<TaskState>()
            .set_toasts(ModelRc::from(Rc::clone(&controller.toasts)));
        ui.global::<TaskState>()
            .set_task_count(controller.tasks.row_count() as i32);
    });

    let weak = ui.as_weak();
    let sink = Arc::new(move |event: TaskEvent| {
        let weak: Weak<App> = weak.clone();
        report::report(&weak, move |ui| apply_event(&ui, event));
    });
    let action: TaskNoticeAction = {
        let weak = ui.as_weak();
        Arc::new(move |target| task_action(&weak, target))
    };
    let service = Rc::new(TaskService {
        center: TaskCenter::new(sink, notices, action),
    });
    for snapshot in service.center.snapshot() {
        apply_event(ui, TaskEvent::Added(snapshot));
    }

    {
        let weak = ui.as_weak();
        let service = Rc::clone(&service);
        ui.global::<TaskState>().on_request_cancel(move |id| {
            if let Some(ui) = weak.upgrade() {
                service.request_cancel(&ui, id);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let service = Rc::clone(&service);
        ui.global::<TaskState>().on_confirm_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                service.confirm_cancel(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let service = Rc::clone(&service);
        ui.global::<TaskState>().on_dismiss_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                service.dismiss_cancel(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<TaskState>().on_activate(move |id| {
            if let Some(ui) = weak.upgrade() {
                activate(&ui, id);
            }
        });
    }
    ui.global::<TaskState>()
        .on_expire_toast(|id| CONTROLLER.with(|controller| controller.remove_toast(id)));
    ui.global::<TaskState>().on_expire_all_toasts(|| {
        CONTROLLER.with(|controller| controller.toasts.set_vec(Vec::new()))
    });

    SERVICE.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&service)));
    service
}

/// The current service. Set up once by [`setup`], before any surface starts a
/// task.
pub(crate) fn service() -> Rc<TaskService> {
    service_if_ready().expect("the task service is set up before any task starts")
}

/// The service, when the app has set it up yet.
///
/// A surface that installs itself before the service — the game view, which
/// `main` builds first — asks this instead of panicking on its first paint.
pub(crate) fn service_if_ready() -> Option<Rc<TaskService>> {
    SERVICE.with(|slot| slot.borrow().clone())
}

fn apply_event(ui: &App, event: TaskEvent) {
    CONTROLLER.with(|controller| {
        match event {
            TaskEvent::Added(snapshot) => {
                controller
                    .targets
                    .borrow_mut()
                    .insert(snapshot.id, snapshot.spec.target.clone());
                apply_content_progress(ui, &snapshot.spec.target, &snapshot.progress, true);
                let row = item(&snapshot);
                controller.tasks.insert(0, row.clone());
                if !ui.global::<NotificationState>().get_visible() {
                    controller.toasts.insert(0, row);
                }
            }
            TaskEvent::Updated(snapshot) => {
                controller
                    .targets
                    .borrow_mut()
                    .insert(snapshot.id, snapshot.spec.target.clone());
                apply_content_progress(ui, &snapshot.spec.target, &snapshot.progress, true);
                let row = item(&snapshot);
                match controller.index_of(snapshot.id) {
                    Some(index) => controller.tasks.set_row_data(index, row.clone()),
                    // An update should follow its add, but a slow event loop
                    // must not lose the card if it does not.
                    None => controller.tasks.insert(0, row.clone()),
                }
                controller.update_toast(snapshot.id, row);
            }
            TaskEvent::Removed { id, .. } => {
                let target = controller.targets.borrow().get(&id).cloned();
                if let Some(index) = controller.index_of(id) {
                    controller.tasks.remove(index);
                }
                controller.close_toast(id);
                controller.targets.borrow_mut().remove(&id);
                if let Some(target) = target {
                    apply_content_progress(ui, &target, &TaskProgress::Indeterminate, false);
                }
                if controller.pending_cancel.get() == id {
                    controller.set_pending(-1);
                    close_confirmation(ui);
                }
            }
        }
        ui.global::<TaskState>()
            .set_task_count(controller.tasks.row_count() as i32);
    });
    notify(ui);
}

/// Mirrors a content task's progress into an open detail panel so a panel
/// reopened during a download sees the same numbers as the task card.
fn apply_content_progress(ui: &App, target: &TaskTarget, progress: &TaskProgress, operating: bool) {
    let TaskTarget::Content { project_id, .. } = target else {
        return;
    };
    let state = ui.global::<ContentState>();
    if state.get_open_detail() == "" || state.get_detail_id().as_str() != project_id {
        return;
    }
    state.set_detail_operating(operating);
    if !operating {
        state.set_detail_progress(0.0);
        state.set_detail_progress_max(0.0);
        state.set_detail_progress_text(SharedString::default());
        return;
    }
    let (value, max, indeterminate) = progress_fields(progress);
    state.set_detail_progress(value);
    state.set_detail_progress_max(max);
    state.set_detail_progress_text(
        if indeterminate {
            String::new()
        } else {
            format!("{} / {}", bytes(value as u64), bytes(max as u64))
        }
        .into(),
    );
}

fn item(snapshot: &shared::task::TaskSnapshot) -> TaskItem {
    let (value, max, indeterminate) = progress_fields(&snapshot.progress);
    TaskItem {
        id: snapshot.id,
        kind: snapshot.spec.kind.key().into(),
        subject: SharedString::from(snapshot.spec.subject.as_str()),
        context: SharedString::from(snapshot.spec.context.as_str()),
        stage: snapshot.stage.key().into(),
        value,
        max,
        indeterminate,
        queued: snapshot.state == shared::task::TaskState::Queued,
        cancelling: snapshot.cancelling,
        closing: false,
        icon: kind_icon(snapshot.spec.kind).into(),
        action_kind: action_kind(&snapshot.spec.target).into(),
    }
}

fn progress_fields(progress: &TaskProgress) -> (f32, f32, bool) {
    match progress {
        TaskProgress::Indeterminate => (0.0, 0.0, true),
        TaskProgress::Bytes { total: 0, .. } => (0.0, 0.0, true),
        TaskProgress::Bytes { completed, total } => (*completed as f32, *total as f32, false),
        TaskProgress::Steps { total: 0, .. } => (0.0, 0.0, true),
        TaskProgress::Steps { completed, total } => (*completed as f32, *total as f32, false),
    }
}

fn kind_icon(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::ContentDownload => "download",
        TaskKind::ModpackInstall => "package",
        TaskKind::GameInstall => "minecraft",
        TaskKind::LibraryDownload => "cloud-download",
    }
}

fn action_kind(target: &TaskTarget) -> &'static str {
    match target {
        TaskTarget::Content { .. } => "content",
        TaskTarget::Instance { .. } => "instance",
        TaskTarget::MultiplayerLibrary => "library",
        TaskTarget::None => "none",
    }
}

fn task_action(weak: &Weak<App>, target: &TaskTarget) -> Option<NoticeAction> {
    if matches!(target, TaskTarget::None) {
        return None;
    }
    let target = target.clone();
    let weak = weak.clone();
    Some(Box::new(move || {
        report::report(&weak, move |ui| activate_target(&ui, &target));
    }))
}

fn activate(ui: &App, id: i32) {
    let target = CONTROLLER.with(|controller| controller.targets.borrow().get(&id).cloned());
    if let Some(target) = target {
        activate_target(ui, &target);
    }
}

fn activate_target(ui: &App, target: &TaskTarget) {
    // The target lives behind the centre; close it before navigating, or the
    // task card's own action would open the instance under an open panel.
    ui.global::<NotificationState>().set_visible(false);
    match target {
        TaskTarget::Content {
            instance_id,
            kind,
            platform,
            project_id,
        } => {
            crate::ui::overlays::content::open_task_target(
                ui,
                instance_id,
                kind,
                platform,
                project_id,
            );
        }
        TaskTarget::Instance { instance_id } => {
            select_instance(ui, instance_id);
        }
        TaskTarget::MultiplayerLibrary => {
            ui.global::<MultiplayerState>().invoke_open();
        }
        TaskTarget::None => {}
    }
}

fn select_instance(ui: &App, instance_id: &str) {
    ui.global::<GameState>()
        .invoke_select_instance(instance_id.into());
    if ui.global::<Navigation>().get_current_page() != "game" {
        ui.global::<Navigation>().invoke_navigate("game".into());
    }
}
