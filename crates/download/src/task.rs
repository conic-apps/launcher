// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub enum Step {
    VerifyExistingFiles,
    DownloadFiles,
}

#[derive(Clone, Deserialize, Serialize)]
/// We use this to store the progress of installation task
pub struct Progress {
    pub completed: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
    /// Download progress will start from 1001,
    ///
    /// In this program, the code
    pub step: Arc<Mutex<Step>>,
    pub speed: Arc<AtomicU64>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            completed: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
            speed: Arc::new(AtomicU64::new(0)),
            step: Arc::new(Mutex::new(Step::DownloadFiles)),
        }
    }
}

impl Progress {
    pub fn reset(&self, ordering: Ordering) {
        self.completed.store(0, ordering);
        self.total.store(0, ordering);
        self.speed.store(0, ordering);
    }
}
