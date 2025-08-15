// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Step {
    VerifyExistingFiles,
    DownloadFiles,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// We use this to store the progress of installation task
pub struct Progress {
    pub completed: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
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

impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        self.completed.load(Ordering::SeqCst) == other.completed.load(Ordering::SeqCst)
            && self.total.load(Ordering::SeqCst) == other.total.load(Ordering::SeqCst)
            && self.speed.load(Ordering::SeqCst) == other.speed.load(Ordering::SeqCst)
            && *self.step.lock().expect("") == *other.step.lock().expect("")
    }
}

impl Progress {
    pub fn reset(&self, ordering: Ordering) {
        self.completed.store(0, ordering);
        self.total.store(0, ordering);
        self.speed.store(0, ordering);
    }
}
