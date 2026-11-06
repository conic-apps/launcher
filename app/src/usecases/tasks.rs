// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The task centre: the application's scheduler of long-running work.
//!
//! This is a use case, not a UI concern. It owns the ids, the ordering, the
//! cancellation policy and the completion notices; the Slint frontend only
//! projects its events into `TaskState`. Producers keep reporting through their
//! own `Sink`, and the app-side use case adapts those reports to a [`TaskHandle`].
//!
//! A task is removed when it ends. The centre never keeps finished work: a
//! completed task becomes a notification, which is the existing record of
//! something that has already happened.
//!
//! `submit` hands the centre both the description and the work. Nothing runs
//! until the centre admits it: a task waits while its lane is full
//! ([`TaskKind::lane`]), which is what keeps one game install from racing
//! another. A launch is not scheduled here at all — [`TaskCenter::reserve_exclusive`]
//! lets it wait for the centre to go idle and then hold the exclusive lane, so
//! it has a screen and a cancel button rather than a card in the queue.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use shared::{
    notification::{Notice, NoticeAction, NoticeKind, NoticeSink},
    task::{
        TaskCancel, TaskEvent, TaskId, TaskKind, TaskLane, TaskOutcome, TaskProgress, TaskSink,
        TaskSnapshot, TaskSpec, TaskStage, TaskState, TaskTarget,
    },
};

/// How long a task may run before stopping it asks the user first.
pub(crate) const CANCEL_CONFIRM_AFTER: Duration = Duration::from_secs(10);

/// Builds the click action a completed task's notice should carry.
///
/// The UI layer supplies this because navigation is a frontend concern; the
/// task centre only knows the task's target.
pub(crate) type TaskNoticeAction = Arc<dyn Fn(&TaskTarget) -> Option<NoticeAction> + Send + Sync>;

/// What a producer hands over for the scheduler to start later.
///
/// Called once, when the task is admitted; the handle it receives is the same
/// one the producer would otherwise have taken from `begin`. `FnOnce` because
/// some work owns its inputs outright — a resolved modpack plan is consumed by
/// the install that runs it.
pub(crate) type TaskWork = Box<dyn FnOnce(TaskHandle) + Send + Sync + 'static>;

/// What the frontend should do with a cancel request.
#[derive(Debug, PartialEq)]
pub(crate) enum CancelDecision {
    /// The task was stopped without asking.
    Immediate,
    /// The task has run long enough that the frontend must ask first.
    NeedsConfirm(Box<TaskSnapshot>),
    /// The id is no longer active.
    Unknown,
}

struct Record {
    snapshot: TaskSnapshot,
    /// Set at admission, so the confirmation grace measures the run and not the
    /// wait in the queue.
    started_at: Instant,
    cancel: Option<TaskCancel>,
    /// Taken when the task is admitted; a queued task still carries its work, so
    /// cancelling it drops the closure and nothing ever runs.
    work: Option<TaskWork>,
}

struct Inner {
    next_id: TaskId,
    tasks: HashMap<TaskId, Record>,
    /// Submission order. A task is removed from here when it is admitted.
    queue: VecDeque<TaskId>,
    /// Taken by work the centre does not schedule — a launch. While it is set,
    /// no exclusive task is admitted, so a launch in flight and an install
    /// cannot overlap.
    exclusive_reserved: bool,
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

/// The exclusive lane, held by work the centre does not schedule.
///
/// Dropping it gives the lane back and admits whatever queued meanwhile, so the
/// guard has to live exactly as long as the launch that took it.
pub(crate) struct TaskReservation {
    center: Arc<TaskCenter>,
}

impl Drop for TaskReservation {
    fn drop(&mut self) {
        self.center.release_exclusive();
    }
}

impl TaskCenter {
    pub(crate) fn new(sink: TaskSink, notices: NoticeSink, action: TaskNoticeAction) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                next_id: 0,
                tasks: HashMap::new(),
                queue: VecDeque::new(),
                exclusive_reserved: false,
            }),
            sink,
            notices,
            action,
        })
    }

    /// Registers a task and reports it as queued.
    ///
    /// `work` is called when the task is admitted — immediately when the lane is
    /// free, and after the task ahead of it ends otherwise.
    pub(crate) fn submit(self: &Arc<Self>, spec: TaskSpec, work: TaskWork) -> TaskId {
        let (id, snapshot) = {
            let mut inner = self.lock();
            let id = inner.next_id;
            inner.next_id = inner.next_id.wrapping_add(1);
            let snapshot = TaskSnapshot {
                id,
                spec,
                state: TaskState::Queued,
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
                    work: Some(work),
                },
            );
            inner.queue.push_back(id);
            (id, snapshot)
        };
        (self.sink)(TaskEvent::Added(snapshot));
        self.drain();
        id
    }

    /// The oldest active task of `kind` that targets `instance_id`.
    ///
    /// Lets a surface ask “is this instance already installing?” without keeping
    /// a second copy of the registry.
    pub(crate) fn active_on(&self, kind: TaskKind, instance_id: &str) -> Option<TaskSnapshot> {
        let inner = self.lock();
        let mut matches: Vec<&Record> = inner
            .tasks
            .values()
            .filter(|record| {
                record.snapshot.spec.kind == kind
                    && instance_of(&record.snapshot.spec.target) == Some(instance_id)
            })
            .collect();
        matches.sort_by_key(|record| record.snapshot.id);
        matches.first().map(|record| record.snapshot.clone())
    }

    /// Takes the exclusive lane for work the centre does not schedule.
    ///
    /// `None` while any task is still active, or while the lane is already
    /// reserved: the caller waits for the centre to go idle and asks again. The
    /// returned guard keeps every exclusive task queued until it is dropped.
    pub(crate) fn reserve_exclusive(self: &Arc<Self>) -> Option<TaskReservation> {
        let mut inner = self.lock();
        if inner.exclusive_reserved || !inner.tasks.is_empty() {
            return None;
        }
        inner.exclusive_reserved = true;
        Some(TaskReservation {
            center: Arc::clone(self),
        })
    }

    fn release_exclusive(self: &Arc<Self>) {
        {
            let mut inner = self.lock();
            inner.exclusive_reserved = false;
        }
        self.drain();
    }

    /// Stores how to stop a task.
    ///
    /// Called right after the work is spawned, so the cancel button cannot be
    /// clicked before there is something to abort.
    pub(crate) fn attach_cancel(&self, id: TaskId, cancel: TaskCancel) {
        if let Some(record) = self.lock().tasks.get_mut(&id)
            && !record.snapshot.cancelling
        {
            record.cancel = Some(cancel);
        }
    }

    /// Requests a stop, applying the elapsed-time confirmation policy.
    pub(crate) fn request_cancel(self: &Arc<Self>, id: TaskId) -> CancelDecision {
        self.request_cancel_at(id, Instant::now())
    }

    fn request_cancel_at(self: &Arc<Self>, id: TaskId, now: Instant) -> CancelDecision {
        {
            let inner = self.lock();
            let Some(record) = inner.tasks.get(&id) else {
                return CancelDecision::Unknown;
            };
            if record.snapshot.cancelling {
                return CancelDecision::Unknown;
            }
            // A task that has not started has nothing to confirm: it owns no
            // half-written file yet.
            if record.snapshot.state == TaskState::Running
                && now.saturating_duration_since(record.started_at) >= CANCEL_CONFIRM_AFTER
            {
                return CancelDecision::NeedsConfirm(Box::new(record.snapshot.clone()));
            }
        }
        self.cancel(id);
        CancelDecision::Immediate
    }

    /// Stops a task immediately, regardless of how long it has run.
    pub(crate) fn cancel_now(self: &Arc<Self>, id: TaskId) {
        self.cancel(id);
    }

    fn cancel(self: &Arc<Self>, id: TaskId) {
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

    /// Returns the tasks that are still active, queued ones included.
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

    fn finish(self: &Arc<Self>, id: TaskId, outcome: TaskOutcome) {
        self.remove(id, outcome);
    }

    /// Admits every queued task that may start, in submission order.
    ///
    /// The work is called with no lock held: a producer's closure spawns its
    /// task and immediately attaches its cancel hook, which takes the lock
    /// again.
    fn drain(self: &Arc<Self>) {
        loop {
            let Some((id, snapshot, work)) = self.admit_next() else {
                return;
            };
            (self.sink)(TaskEvent::Updated(snapshot));
            work(TaskHandle {
                id,
                center: Arc::clone(self),
            });
        }
    }

    /// Moves the first runnable queued task to `Running`.
    ///
    /// The queue is scanned rather than read from the front: a task waiting for
    /// the exclusive lane must not hold up a parallel download queued behind it.
    fn admit_next(&self) -> Option<(TaskId, TaskSnapshot, TaskWork)> {
        let mut inner = self.lock();
        let index = {
            let Inner {
                tasks,
                queue,
                exclusive_reserved,
                ..
            } = &mut *inner;
            let mut found = None;
            for (index, id) in queue.iter().enumerate() {
                let Some(record) = tasks.get(id) else {
                    continue;
                };
                if runnable(tasks, *exclusive_reserved, record) {
                    found = Some(index);
                    break;
                }
            }
            found
        }?;
        let id = inner.queue.remove(index)?;
        let record = inner.tasks.get_mut(&id)?;
        record.snapshot.state = TaskState::Running;
        record.started_at = Instant::now();
        let snapshot = record.snapshot.clone();
        let work = record.work.take()?;
        Some((id, snapshot, work))
    }

    fn remove(self: &Arc<Self>, id: TaskId, outcome: TaskOutcome) {
        let spec = {
            let mut inner = self.lock();
            let Some(record) = inner.tasks.get(&id) else {
                return;
            };
            // A cancel already decided this task's outcome.
            if record.snapshot.cancelling && outcome != TaskOutcome::Cancelled {
                return;
            }
            inner.queue.retain(|queued| *queued != id);
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
        self.drain();
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Whether a queued task may start now.
fn runnable(tasks: &HashMap<TaskId, Record>, reserved: bool, record: &Record) -> bool {
    let lane = record.snapshot.spec.kind.lane();
    // A launch holds the lane while it runs; nothing may finish joining it.
    if lane == TaskLane::Exclusive && reserved {
        return false;
    }
    let running = tasks
        .values()
        .filter(|other| {
            other.snapshot.state == TaskState::Running && other.snapshot.spec.kind.lane() == lane
        })
        .count();
    running < lane.capacity()
}

/// The instance a target belongs to, which is what `active_on` matches on.
fn instance_of(target: &TaskTarget) -> Option<&str> {
    match target {
        TaskTarget::Content { instance_id, .. } | TaskTarget::Instance { instance_id } => {
            Some(instance_id)
        }
        TaskTarget::MultiplayerLibrary | TaskTarget::None => None,
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
    use std::sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use shared::{
        notification::Notice,
        task::{TaskEvent, TaskSpec, TaskTarget},
    };

    use super::*;

    fn spec() -> TaskSpec {
        TaskSpec {
            kind: TaskKind::ContentDownload,
            subject: "Sodium".into(),
            context: "1.21".into(),
            target: TaskTarget::None,
        }
    }

    fn install(instance: &str) -> TaskSpec {
        TaskSpec {
            kind: TaskKind::GameInstall,
            subject: instance.into(),
            context: "1.21".into(),
            target: TaskTarget::Instance {
                instance_id: instance.into(),
            },
        }
    }

    /// Work that records that it ran and returns a cancel hook that records it.
    fn counted(runs: &Arc<AtomicUsize>, cancelled: &Arc<AtomicBool>) -> TaskWork {
        let runs = Arc::clone(runs);
        let cancelled = Arc::clone(cancelled);
        Box::new(move |task: TaskHandle| {
            runs.fetch_add(1, Ordering::SeqCst);
            let id = task.id();
            let cancelled = Arc::clone(&cancelled);
            task.center.attach_cancel(
                id,
                Arc::new(move || cancelled.store(true, Ordering::SeqCst)),
            );
        })
    }

    fn noop() -> TaskWork {
        Box::new(|_| {})
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
    fn submitting_reports_the_task_then_its_admission() {
        let (center, events, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        let id = center.submit(spec(), counted(&runs, &cancelled));

        let events = lock(&events);
        assert_eq!(events.len(), 2);
        let TaskEvent::Added(added) = &events[0] else {
            panic!("expected an added event");
        };
        assert_eq!(added.id, id);
        assert_eq!(added.state, TaskState::Queued);
        assert_eq!(added.stage, TaskStage::Preparing);
        let TaskEvent::Updated(running) = &events[1] else {
            panic!("expected an update");
        };
        assert_eq!(running.state, TaskState::Running);
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_second_exclusive_task_waits_for_the_first() {
        let (center, _, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        let first = center.submit(install("a"), counted(&runs, &cancelled));
        let second = center.submit(install("b"), counted(&runs, &cancelled));

        assert_eq!(runs.load(Ordering::SeqCst), 1);
        let queued = center
            .snapshot()
            .into_iter()
            .find(|task| task.id == second)
            .expect("the second task is still active");
        assert_eq!(queued.state, TaskState::Queued);

        center.cancel_now(first);
        let admitted = center
            .snapshot()
            .into_iter()
            .find(|task| task.id == second)
            .expect("still active");
        assert_eq!(admitted.state, TaskState::Running);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_parallel_download_runs_beside_an_exclusive_install() {
        let (center, _, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));

        center.submit(install("a"), counted(&runs, &cancelled));
        center.submit(spec(), counted(&runs, &cancelled));
        center.submit(spec(), counted(&runs, &cancelled));

        assert_eq!(runs.load(Ordering::SeqCst), 3);
        assert!(
            center
                .snapshot()
                .iter()
                .all(|task| task.state == TaskState::Running)
        );
    }

    #[test]
    fn a_reservation_waits_for_the_running_task() {
        let (center, _, _) = centre();
        center.submit(install("a"), noop());
        assert!(center.reserve_exclusive().is_none());

        center.cancel_now(center.snapshot()[0].id);
        assert!(center.reserve_exclusive().is_some());
    }

    #[test]
    fn a_reservation_holds_the_lane_until_it_is_dropped() {
        let (center, _, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));

        let reservation = center.reserve_exclusive().expect("the lane is free");
        let id = center.submit(install("a"), counted(&runs, &cancelled));
        assert_eq!(runs.load(Ordering::SeqCst), 0);
        assert_eq!(
            center
                .snapshot()
                .into_iter()
                .find(|task| task.id == id)
                .expect("queued")
                .state,
            TaskState::Queued
        );

        drop(reservation);
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_reservation_does_not_hold_back_a_parallel_download() {
        let (center, _, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));

        let _reservation = center.reserve_exclusive().expect("the lane is free");
        center.submit(spec(), counted(&runs, &cancelled));
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_queued_task_never_starts_when_cancelled() {
        let (center, events, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        center.submit(install("a"), counted(&runs, &cancelled));
        let queued = center.submit(install("b"), counted(&runs, &cancelled));

        assert_eq!(center.request_cancel(queued), CancelDecision::Immediate);
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert!(matches!(
            lock(&events).last(),
            Some(TaskEvent::Removed {
                outcome: TaskOutcome::Cancelled,
                ..
            })
        ));
    }

    #[test]
    fn duplicate_progress_is_dropped() {
        let (center, events, _) = centre();
        let task = TaskHandle {
            id: center.submit(spec(), noop()),
            center: Arc::clone(&center),
        };
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
        assert_eq!(lock(&events).len(), 3);
    }

    #[test]
    fn finishing_removes_the_task_and_posts_a_notice() {
        let (center, events, notices) = centre();
        let task = TaskHandle {
            id: center.submit(spec(), noop()),
            center: Arc::clone(&center),
        };
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
        let task = TaskHandle {
            id: center.submit(
                TaskSpec {
                    kind: TaskKind::ContentDownload,
                    subject: "Sodium".into(),
                    context: "1.21".into(),
                    target: TaskTarget::Content {
                        instance_id: "abc".into(),
                        kind: "mods".into(),
                        platform: "modrinth".into(),
                        project_id: "sodium".into(),
                    },
                },
                noop(),
            ),
            center: Arc::clone(&center),
        };
        task.succeed();

        let mut notices = lock(&notices);
        let action = notices[0].action.take().expect("an action");
        action();
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn failing_removes_the_task_and_posts_the_detail() {
        let (center, events, notices) = centre();
        let task = TaskHandle {
            id: center.submit(spec(), noop()),
            center: Arc::clone(&center),
        };
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
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        let id = center.submit(install("a"), counted(&runs, &cancelled));

        assert_eq!(center.request_cancel(id), CancelDecision::Immediate);
        assert!(cancelled.load(Ordering::SeqCst));
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
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        let id = center.submit(install("a"), counted(&runs, &cancelled));

        let decision = center.request_cancel_at(
            id,
            Instant::now() + CANCEL_CONFIRM_AFTER + Duration::from_millis(1),
        );
        assert!(matches!(decision, CancelDecision::NeedsConfirm(_)));
        assert!(!cancelled.load(Ordering::SeqCst));
        assert_eq!(center.snapshot().len(), 1);
        assert_eq!(lock(&events).len(), 2);
        assert!(lock(&notices).is_empty());

        center.cancel_now(id);
        assert!(cancelled.load(Ordering::SeqCst));
        assert!(center.snapshot().is_empty());
    }

    #[test]
    fn a_queued_task_never_waits_for_a_confirmation() {
        let (center, _, _) = centre();
        let runs = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        // The first install holds the lane while the second waits in the queue.
        center.submit(install("a"), noop());
        let waiting = center.submit(install("b"), counted(&runs, &cancelled));

        // Nothing has run yet, so there is nothing to confirm however long it
        // has been waiting.
        let decision =
            center.request_cancel_at(waiting, Instant::now() + Duration::from_secs(3600));
        assert_eq!(decision, CancelDecision::Immediate);
        assert!(!cancelled.load(Ordering::SeqCst));
        assert_eq!(runs.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn late_progress_after_cancelling_is_dropped() {
        let (center, events, _) = centre();
        let task = TaskHandle {
            id: center.submit(spec(), noop()),
            center: Arc::clone(&center),
        };
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
        let task = TaskHandle {
            id: center.submit(spec(), noop()),
            center: Arc::clone(&center),
        };
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
    fn active_on_finds_the_instance_task_and_ignores_the_rest() {
        let (center, _, _) = centre();
        center.submit(install("a"), noop());
        center.submit(spec(), noop());

        let found = center.active_on(TaskKind::GameInstall, "a");
        assert_eq!(found.expect("an install for a").spec.subject, "a");
        assert!(center.active_on(TaskKind::GameInstall, "b").is_none());
        assert!(center.active_on(TaskKind::ModpackInstall, "a").is_none());
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
