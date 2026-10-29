// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! UI-neutral vocabulary for the task centre.
//!
//! A task is long-running work the user started and can still see or stop: a
//! content download, a modpack install, the game install behind a launch. It
//! crosses the boundary as plain data, so the use case that owns its lifetime
//! mentions neither Slint nor the app, and a producer only needs a [`TaskSink`].
//!
//! The kind selects the frontend's translated title and icon; the target says
//! where a click on the card goes. Both are data rather than a callback: a task
//! card can be clicked more than once, unlike a one-shot notification action.

use crate::Sink;

/// A task's identity for as long as it is running.
pub type TaskId = i32;

/// What the task is doing, which selects the frontend's title and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    /// A mod, resource pack or shader pack being written into an instance.
    ContentDownload,
    /// A resolved modpack's instance being created and filled.
    ModpackInstall,
    /// The Minecraft, Java or loader install behind a launch.
    GameInstall,
    /// The Conic Nexus dynamic library.
    LibraryDownload,
}

impl TaskKind {
    /// The stable key the frontend branches on.
    pub fn key(self) -> &'static str {
        match self {
            Self::ContentDownload => "content-download",
            Self::ModpackInstall => "modpack-install",
            Self::GameInstall => "game-install",
            Self::LibraryDownload => "library-download",
        }
    }
}

/// The step the task is currently on. The frontend translates it.
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
    /// The stable key the frontend branches on.
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
    Launch {
        instance_id: String,
    },
    MultiplayerLibrary,
    None,
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
/// the launch flow — so the centre never mentions tokio or a specific producer.
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
}
