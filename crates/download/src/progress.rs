// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum DownloadPhase {
    VerifyExistingFiles,
    DownloadFiles,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// We use this to store the progress of installation task
pub struct DownloadState {
    pub completed: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
    pub phase: Arc<Mutex<DownloadPhase>>,
    pub speed: Arc<AtomicU64>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            completed: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
            speed: Arc::new(AtomicU64::new(0)),
            phase: Arc::new(Mutex::new(DownloadPhase::DownloadFiles)),
        }
    }
}

impl PartialEq for DownloadState {
    fn eq(&self, other: &Self) -> bool {
        self.completed.load(Ordering::SeqCst) == other.completed.load(Ordering::SeqCst)
            && self.total.load(Ordering::SeqCst) == other.total.load(Ordering::SeqCst)
            && self.speed.load(Ordering::SeqCst) == other.speed.load(Ordering::SeqCst)
            && *self.phase.lock().expect("") == *other.phase.lock().expect("")
    }
}

impl DownloadState {
    pub fn reset(&self, ordering: Ordering) {
        self.completed.store(0, ordering);
        self.total.store(0, ordering);
        self.speed.store(0, ordering);
    }
}
