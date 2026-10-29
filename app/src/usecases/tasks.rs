// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The task centre: the application's registry of running work.
//!
//! This is a use case, not a UI concern. It owns the ids, the lifecycle, the
//! cancel policy and the completion notices; the Slint frontend only projects
//! its events into `TaskState`. Producers keep reporting through their own
//! `Sink`, and the app-side use case adapts those reports to a [`TaskHandle`].
//!
//! A task is removed when it ends. The centre never keeps finished work: a
//! completed task becomes a notification, which is the existing record of
//! something that has already happened.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use shared::{
    notification::{Notice, NoticeAction, NoticeKind, NoticeSink},
    task::{
        TaskCancel, TaskEvent, TaskId, TaskOutcome, TaskProgress, TaskSink, TaskSnapshot, TaskSpec,
        TaskStage, TaskTarget,
    },
};

/// How long a task may run before stopping it asks the user first.
pub(crate) const CANCEL_CONFIRM_AFTER: Duration = Duration::from_secs(10);

/// Builds the click action a completed task's notice should carry.
///
/// The UI layer supplies this because navigation is a frontend concern; the
/// task centre only knows the task's target.
pub(crate) type TaskNoticeAction = Arc<dyn Fn(&TaskTarget) -> Option<NoticeAction> + Send + Sync>;

/// What the frontend should do with a cancel request.
#[derive(Debug, PartialEq)]
pub(crate) enum CancelDecision {
    /// The task was under the threshold and has been stopped.
    Immediate,
    /// The task has run long enough that the frontend must ask first.
    NeedsConfirm(TaskSnapshot),
    /// The id is no longer active.
    Unknown,
}

struct Record {
    snapshot: TaskSnapshot,
    started_at: Instant,
    cancel: Option<TaskCancel>,
}

struct Inner {
    next_id: TaskId,
    tasks: HashMap<TaskId, Record>,
}

/// The one registry of running tasks.
pub(crate) struct TaskCenter {
    inner: Mutex<Inner>,
    sink: TaskSink,
    notices: NoticeSink,
    action: TaskNoticeAction,
}

/// A producer's handle to one task.
///
/// Cloneable and `Send`: a use case can keep one in each branch of its work.
/// Finishing is idempotent, and an event for an id that is no longer active is
/// ignored, which is what makes a late report after cancellation harmless.
#[derive(Clone)]
pub(crate) struct TaskHandle {
    id: TaskId,
    center: Arc<TaskCenter>,
}

impl TaskHandle {
    pub(crate) fn id(&self) -> TaskId {
        self.id
    }

    /// Records a new stage and its progress in one update.
    pub(crate) fn update(&self, stage: TaskStage, progress: TaskProgress) {
        self.center.update(self.id, stage, progress);
    }

    /// Replaces the card's click target, e.g. once a modpack's instance exists.
    pub(crate) fn retarget(&self, target: TaskTarget) {
        self.center.retarget(self.id, target);
    }

    /// Ends the task successfully.
    pub(crate) fn succeed(&self) {
        self.center.finish(self.id, TaskOutcome::Completed);
    }

    /// Ends the task with a failure detail.
    pub(crate) fn fail(&self, message: impl Into<String>) {
        self.center
            .finish(self.id, TaskOutcome::Failed(message.into()));
    }
}

impl TaskCenter {
    pub(crate) fn new(sink: TaskSink, notices: NoticeSink, action: TaskNoticeAction) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                next_id: 0,
                tasks: HashMap::new(),
            }),
            sink,
            notices,
            action,
        })
    }

    /// Registers a task and reports it as added.
    pub(crate) fn begin(self: &Arc<Self>, spec: TaskSpec) -> TaskHandle {
        let (id, snapshot) = {
            let mut inner = self.lock();
            let id = inner.next_id;
            inner.next_id = inner.next_id.wrapping_add(1);
            let snapshot = TaskSnapshot {
                id,
                spec,
                stage: TaskStage::Preparing,
                progress: TaskProgress::Indeterminate,
                cancelling: false,
            };
            inner.tasks.insert(
                id,
                Record {
                    snapshot: snapshot.clone(),
                    started_at: Instant::now(),
                    cancel: None,
                },
            );
            (id, snapshot)
        };
        (self.sink)(TaskEvent::Added(snapshot));
        TaskHandle {
            id,
            center: Arc::clone(self),
        }
    }

    /// Stores how to stop a task.
    ///
    /// Called on the UI thread right after the work is spawned, so the cancel
    /// button cannot be clicked before there is something to abort.
    pub(crate) fn attach_cancel(&self, id: TaskId, cancel: TaskCancel) {
        if let Some(record) = self.lock().tasks.get_mut(&id)
            && !record.snapshot.cancelling
        {
            record.cancel = Some(cancel);
        }
    }

    /// Requests a stop, applying the elapsed-time confirmation policy.
    pub(crate) fn request_cancel(&self, id: TaskId) -> CancelDecision {
        self.request_cancel_at(id, Instant::now())
    }

    fn request_cancel_at(&self, id: TaskId, now: Instant) -> CancelDecision {
        {
            let inner = self.lock();
            let Some(record) = inner.tasks.get(&id) else {
                return CancelDecision::Unknown;
            };
            if record.snapshot.cancelling {
                return CancelDecision::Unknown;
            }
            let elapsed = now.saturating_duration_since(record.started_at);
            if elapsed >= CANCEL_CONFIRM_AFTER {
                return CancelDecision::NeedsConfirm(record.snapshot.clone());
            }
        }
        self.cancel(id);
        CancelDecision::Immediate
    }

    /// Stops a task immediately, regardless of how long it has run.
    pub(crate) fn cancel_now(&self, id: TaskId) {
        self.cancel(id);
    }

    fn cancel(&self, id: TaskId) {
        let (snapshot, cancel) = {
            let mut inner = self.lock();
            let Some(record) = inner.tasks.get_mut(&id) else {
                return;
            };
            if record.snapshot.cancelling {
                return;
            }
            record.snapshot.cancelling = true;
            (record.snapshot.clone(), record.cancel.take())
        };
        (self.sink)(TaskEvent::Updated(snapshot));

        if let Some(cancel) = cancel {
            cancel();
        }
        self.remove(id, TaskOutcome::Cancelled);
    }

    /// Returns the tasks that are still running.
    pub(crate) fn snapshot(&self) -> Vec<TaskSnapshot> {
        let inner = self.lock();
        let mut tasks: Vec<_> = inner.tasks.values().map(|r| r.snapshot.clone()).collect();
        tasks.sort_by_key(|task| task.id);
        tasks
    }

    fn update(&self, id: TaskId, stage: TaskStage, progress: TaskProgress) {
        let snapshot = {
            let mut inner = self.lock();
            let Some(record) = inner.tasks.get_mut(&id) else {
                return;
            };
            // A cancel owns the task's exit; a tick that arrives while the work
            // is being torn down must not resurrect the card.
            if record.snapshot.cancelling {
                return;
            }
            if record.snapshot.stage == stage && record.snapshot.progress == progress {
                return;
            }
            record.snapshot.stage = stage;
            record.snapshot.progress = progress;
            record.snapshot.clone()
        };
        (self.sink)(TaskEvent::Updated(snapshot));
    }

    fn retarget(&self, id: TaskId, target: TaskTarget) {
        let snapshot = {
            let mut inner = self.lock();
            let Some(record) = inner.tasks.get_mut(&id) else {
                return;
            };
            if record.snapshot.cancelling || record.snapshot.spec.target == target {
                return;
            }
            record.snapshot.spec.target = target;
            record.snapshot.clone()
        };
        (self.sink)(TaskEvent::Updated(snapshot));
    }

    fn finish(&self, id: TaskId, outcome: TaskOutcome) {
        self.remove(id, outcome);
    }

    fn remove(&self, id: TaskId, outcome: TaskOutcome) {
        let spec = {
            let mut inner = self.lock();
            let Some(record) = inner.tasks.get(&id) else {
                return;
            };
            // A cancel already decided this task's outcome.
            if record.snapshot.cancelling && outcome != TaskOutcome::Cancelled {
                return;
            }
            let record = inner.tasks.remove(&id);
            record.map(|record| record.snapshot.spec)
        };
        let Some(spec) = spec else {
            return;
        };
        (self.sink)(TaskEvent::Removed {
            id,
            outcome: outcome.clone(),
        });
        (self.notices)(notice_for(&spec, &outcome, &self.action));
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn notice_for(spec: &TaskSpec, outcome: &TaskOutcome, action: &TaskNoticeAction) -> Notice {
    match outcome {
        TaskOutcome::Completed => {
            let mut notice = Notice::new(NoticeKind::TaskFinished)
                .with_arg(spec.subject.clone())
                .with_arg(spec.kind.key());
            notice.action = action(&spec.target);
            notice
        }
        TaskOutcome::Failed(error) => Notice::new(NoticeKind::TaskFailed).with_arg(error),
        TaskOutcome::Cancelled => Notice::new(NoticeKind::TaskCancelled).with_arg(&spec.subject),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Mutex, MutexGuard,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use shared::{
        notification::Notice,
        task::{TaskEvent, TaskSpec, TaskTarget},
    };

    use super::*;

    fn spec() -> TaskSpec {
        TaskSpec {
            kind: shared::task::TaskKind::ContentDownload,
            subject: "Sodium".into(),
            context: "1.21".into(),
            target: TaskTarget::None,
        }
    }

    type Events = Arc<Mutex<Vec<TaskEvent>>>;
    type Notices = Arc<Mutex<Vec<Notice>>>;

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn centre() -> (Arc<TaskCenter>, Events, Notices) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let notices = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let events = Arc::clone(&events);
            Arc::new(move |event| lock(&events).push(event))
        };
        let notice_sink = {
            let notices = Arc::clone(&notices);
            Arc::new(move |notice| lock(&notices).push(notice))
        };
        let action: TaskNoticeAction = Arc::new(|_| None);
        (TaskCenter::new(sink, notice_sink, action), events, notices)
    }

    #[test]
    fn beginning_a_task_reports_it() {
        let (center, events, _) = centre();
        let task = center.begin(spec());

        let events = lock(&events);
        assert_eq!(events.len(), 1);
        let TaskEvent::Added(snapshot) = &events[0] else {
            panic!("expected an added event");
        };
        assert_eq!(snapshot.id, task.id());
        assert_eq!(snapshot.stage, TaskStage::Preparing);
        assert!(!snapshot.cancelling);
    }

    #[test]
    fn duplicate_progress_is_dropped() {
        let (center, events, _) = centre();
        let task = center.begin(spec());
        task.update(
            TaskStage::Downloading,
            TaskProgress::Bytes {
                completed: 1,
                total: 2,
            },
        );
        task.update(
            TaskStage::Downloading,
            TaskProgress::Bytes {
                completed: 1,
                total: 2,
            },
        );
        assert_eq!(lock(&events).len(), 2);
    }

    #[test]
    fn finishing_removes_the_task_and_posts_a_notice() {
        let (center, events, notices) = centre();
        let task = center.begin(spec());
        task.succeed();

        assert!(center.snapshot().is_empty());
        assert!(matches!(
            lock(&events).last(),
            Some(TaskEvent::Removed {
                outcome: TaskOutcome::Completed,
                ..
            })
        ));
        let notices = lock(&notices);
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, NoticeKind::TaskFinished);
        assert_eq!(notices[0].args, ["Sodium", "content-download"]);
        assert!(notices[0].action.is_none());
    }

    #[test]
    fn a_completed_notice_gets_the_task_action() {
        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        let events = Arc::new(Mutex::new(Vec::new()));
        let notices = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let events = Arc::clone(&events);
            Arc::new(move |event| lock(&events).push(event))
        };
        let notice_sink = {
            let notices = Arc::clone(&notices);
            Arc::new(move |notice| lock(&notices).push(notice))
        };
        let action: TaskNoticeAction = Arc::new(move |target| {
            assert_eq!(
                target,
                &TaskTarget::Content {
                    instance_id: "abc".into(),
                    kind: "mods".into(),
                    platform: "modrinth".into(),
                    project_id: "sodium".into(),
                }
            );
            let flag = Arc::clone(&flag);
            Some(Box::new(move || flag.store(true, Ordering::SeqCst)))
        });
        let center = TaskCenter::new(sink, notice_sink, action);
        let task = center.begin(TaskSpec {
            kind: shared::task::TaskKind::ContentDownload,
            subject: "Sodium".into(),
            context: "1.21".into(),
            target: TaskTarget::Content {
                instance_id: "abc".into(),
                kind: "mods".into(),
                platform: "modrinth".into(),
                project_id: "sodium".into(),
            },
        });
        task.succeed();

        let mut notices = lock(&notices);
        let action = notices[0].action.take().expect("an action");
        action();
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn failing_removes_the_task_and_posts_the_detail() {
        let (center, events, notices) = centre();
        let task = center.begin(spec());
        task.fail("network down");

        assert!(center.snapshot().is_empty());
        assert!(matches!(
            lock(&events).last(),
            Some(TaskEvent::Removed {
                outcome: TaskOutcome::Failed(_),
                ..
            })
        ));
        assert_eq!(lock(&notices)[0].args, ["network down"]);
    }

    #[test]
    fn a_short_task_cancels_immediately() {
        let (center, events, notices) = centre();
        let task = center.begin(spec());
        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        center.attach_cancel(
            task.id(),
            Arc::new(move || flag.store(true, Ordering::SeqCst)),
        );

        assert_eq!(center.request_cancel(task.id()), CancelDecision::Immediate);
        assert!(fired.load(Ordering::SeqCst));
        assert!(center.snapshot().is_empty());
        assert!(matches!(
            lock(&events).last(),
            Some(TaskEvent::Removed {
                outcome: TaskOutcome::Cancelled,
                ..
            })
        ));
        assert_eq!(lock(&notices)[0].kind, NoticeKind::TaskCancelled);
    }

    #[test]
    fn a_long_task_asks_before_cancelling() {
        let (center, events, notices) = centre();
        let task = center.begin(spec());
        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        center.attach_cancel(
            task.id(),
            Arc::new(move || flag.store(true, Ordering::SeqCst)),
        );

        let decision = center.request_cancel_at(
            task.id(),
            Instant::now() + CANCEL_CONFIRM_AFTER + Duration::from_millis(1),
        );
        assert!(matches!(decision, CancelDecision::NeedsConfirm(_)));
        assert!(!fired.load(Ordering::SeqCst));
        assert_eq!(center.snapshot().len(), 1);
        assert_eq!(lock(&events).len(), 1);
        assert!(lock(&notices).is_empty());

        center.cancel_now(task.id());
        assert!(fired.load(Ordering::SeqCst));
        assert!(center.snapshot().is_empty());
    }

    #[test]
    fn late_progress_after_cancelling_is_dropped() {
        let (center, events, _) = centre();
        let task = center.begin(spec());
        center.cancel_now(task.id());

        task.update(
            TaskStage::Downloading,
            TaskProgress::Bytes {
                completed: 1,
                total: 2,
            },
        );
        task.succeed();
        assert!(center.snapshot().is_empty());
        assert!(matches!(
            lock(&events).last(),
            Some(TaskEvent::Removed { .. })
        ));
    }

    #[test]
    fn retargeting_updates_the_card_target() {
        let (center, events, _) = centre();
        let task = center.begin(spec());
        task.retarget(TaskTarget::Instance {
            instance_id: "abc".into(),
        });

        let events = lock(&events);
        let TaskEvent::Updated(snapshot) = events.last().expect("an update") else {
            panic!("expected an update");
        };
        assert_eq!(
            snapshot.spec.target,
            TaskTarget::Instance {
                instance_id: "abc".into()
            }
        );
    }

    #[test]
    fn unknown_ids_are_ignored() {
        let (center, _, _) = centre();
        assert_eq!(center.request_cancel(999), CancelDecision::Unknown);
        center.cancel_now(999);
        center.update(999, TaskStage::Downloading, TaskProgress::Indeterminate);
        center.finish(999, TaskOutcome::Completed);
    }
}
