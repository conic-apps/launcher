// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{atomic::AtomicUsize, Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::MAIN_WINDOW;

/// Define the task type
#[derive(Clone, Deserialize, Serialize)]
pub enum Task {
    Chore,
    PrepareInstallGame,
    DownloadFiles,
    VerifyExistingFiles,
    InstallJava,
    InstallModLoader,
}

#[derive(Clone, Deserialize, Serialize)]
/// We use this to store the progress of installation task
pub struct Progress {
    pub completed: Arc<AtomicUsize>,
    pub total: Arc<AtomicUsize>,
    /// Download progress will start from 1001,
    ///
    /// In this program, the code
    pub task: Arc<Mutex<Task>>,
    pub speed: Arc<AtomicUsize>,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            completed: Arc::new(AtomicUsize::new(0)),
            total: Arc::new(AtomicUsize::new(0)),
            speed: Arc::new(AtomicUsize::new(0)),
            task: Arc::new(Mutex::new(Task::Chore)),
        }
    }

    pub fn send(&self) {
        MAIN_WINDOW.emit("task_progress", self).unwrap();
    }
}
