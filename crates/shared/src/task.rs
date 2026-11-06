// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! UI-neutral vocabulary for the task centre: long-running, cancellable work
//! such as a content download or a game install.
//!
//! The centre schedules that work; this module carries the words it schedules
//! with. A launch is deliberately *not* one of them: it is a screen with its own
//! cancel button, so it waits for the centre to go idle and reserves the
//! exclusive lane instead of joining the queue.

use crate::Sink;

pub type TaskId = i32;

/// What the task is doing, which selects the frontend's title and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    /// A mod, resource pack or shader pack being written into an instance.
    ContentDownload,
    /// A resolved modpack's instance being created and filled.
    ModpackInstall,
    /// The Minecraft, Java or loader install of an instance, including a repair.
    GameInstall,
    /// The Conic Nexus dynamic library.
    LibraryDownload,
}

/// Whether a task may run beside other tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskLane {
    /// One at a time across the whole app. Every task that rewrites an
    /// instance's game files lives here, so two installs can never interleave —
    /// and a launch, which reserves this lane for itself, can never overlap one.
    Exclusive,
    /// As many as the producers start. Downloads that only add files to an
    /// instance have no reason to wait for one another.
    Parallel,
}

impl TaskLane {
    pub fn capacity(self) -> usize {
        match self {
            Self::Exclusive => 1,
            Self::Parallel => usize::MAX,
        }
    }
}

impl TaskKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::ContentDownload => "content-download",
            Self::ModpackInstall => "modpack-install",
            Self::GameInstall => "game-install",
            Self::LibraryDownload => "library-download",
        }
    }

    pub fn lane(self) -> TaskLane {
        match self {
            // Everything that rewrites an instance's game files: a second one
            // would fight the first over the same paths.
            Self::GameInstall | Self::ModpackInstall => TaskLane::Exclusive,
            Self::ContentDownload | Self::LibraryDownload => TaskLane::Parallel,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStage {
    Preparing,
    Verifying,
    Downloading,
    InstallingGame,
    InstallingJava,
    InstallingLoader,
    Extracting,
    Finalizing,
}

impl TaskStage {
    pub fn key(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Verifying => "verifying",
            Self::Downloading => "downloading",
            Self::InstallingGame => "installing-game",
            Self::InstallingJava => "installing-java",
            Self::InstallingLoader => "installing-loader",
            Self::Extracting => "extracting",
            Self::Finalizing => "finalizing",
        }
    }
}

/// How far the current stage has got.
///
/// `Bytes` with a zero total and `Indeterminate` both draw the sweeping bar; a
/// task whose size is not known yet must not divide by zero in the frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TaskProgress {
    #[default]
    Indeterminate,
    Bytes {
        completed: u64,
        total: u64,
    },
    Steps {
        completed: u32,
        total: u32,
    },
}

/// Where a click on the task card goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskTarget {
    Content {
        instance_id: String,
        kind: String,
        /// "modrinth" | "curseforge"
        platform: String,
        project_id: String,
    },
    Instance {
        instance_id: String,
    },
    MultiplayerLibrary,
    None,
}

/// Where a task is in its life.
///
/// A queued task has been submitted but its producer's work has not started: no
/// file has been touched and nothing is on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
}

/// The immutable part of a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSpec {
    pub kind: TaskKind,
    /// The project, pack, library or version name. Data, never a sentence.
    pub subject: String,
    /// The instance name or another short context line, empty when there is none.
    pub context: String,
    pub target: TaskTarget,
}

/// A task as the frontend sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub spec: TaskSpec,
    pub state: TaskState,
    pub stage: TaskStage,
    pub progress: TaskProgress,
    /// The user asked to stop it and the work has not acknowledged that yet.
    pub cancelling: bool,
}

/// How a task left the active list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskOutcome {
    Completed,
    Failed(String),
    Cancelled,
}

/// What the task centre reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskEvent {
    Added(TaskSnapshot),
    Updated(TaskSnapshot),
    Removed { id: TaskId, outcome: TaskOutcome },
}

/// The output port producers report a task through.
pub type TaskSink = Sink<TaskEvent>;

/// The input port through which the task centre stops a running task.
///
/// The work owns the implementation — aborting its `JoinHandle`, or cancelling
/// a flow — so the centre never mentions tokio or a specific producer.
pub type TaskCancel = std::sync::Arc<dyn Fn() + Send + Sync + 'static>;

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send + Sync>() {}

    #[test]
    fn a_task_event_can_cross_a_thread_boundary() {
        assert_send::<TaskEvent>();
        assert_send::<TaskSnapshot>();
        assert_send::<TaskCancel>();
    }

    #[test]
    fn stages_have_stable_frontend_keys() {
        assert_eq!(TaskStage::Preparing.key(), "preparing");
        assert_eq!(TaskStage::InstallingLoader.key(), "installing-loader");
        assert_eq!(TaskStage::Finalizing.key(), "finalizing");
    }

    #[test]
    fn kinds_have_stable_frontend_keys() {
        assert_eq!(TaskKind::ContentDownload.key(), "content-download");
        assert_eq!(TaskKind::ModpackInstall.key(), "modpack-install");
        assert_eq!(TaskKind::GameInstall.key(), "game-install");
        assert_eq!(TaskKind::LibraryDownload.key(), "library-download");
    }

    #[test]
    fn only_the_install_kinds_take_the_exclusive_lane() {
        for kind in [TaskKind::GameInstall, TaskKind::ModpackInstall] {
            assert_eq!(kind.lane(), TaskLane::Exclusive, "{kind:?}");
            assert_eq!(kind.lane().capacity(), 1, "{kind:?}");
        }
        for kind in [TaskKind::ContentDownload, TaskKind::LibraryDownload] {
            assert_eq!(kind.lane(), TaskLane::Parallel, "{kind:?}");
            assert!(kind.lane().capacity() > 1, "{kind:?}");
        }
    }
}
