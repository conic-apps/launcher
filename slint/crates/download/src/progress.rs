// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub enum DownloadPhase {
    VerifyExistingFiles,
    #[default]
    DownloadFiles,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
/// We use this to store the progress of installation task
pub struct DownloadState {
    pub completed_tasks: Arc<AtomicU64>,
    pub total_tasks: Arc<AtomicU64>,
    pub completed_bytes: Arc<AtomicU64>,
    pub total_bytes: Arc<AtomicU64>,
    pub phase: Arc<Mutex<DownloadPhase>>,
    pub speed: Arc<AtomicU64>,
}

impl PartialEq for DownloadState {
    fn eq(&self, other: &Self) -> bool {
        self.completed_tasks.load(Ordering::SeqCst) == other.completed_tasks.load(Ordering::SeqCst)
            && self.total_tasks.load(Ordering::SeqCst) == other.total_tasks.load(Ordering::SeqCst)
            && self.completed_bytes.load(Ordering::SeqCst)
                == other.completed_bytes.load(Ordering::SeqCst)
            && self.total_bytes.load(Ordering::SeqCst) == other.total_bytes.load(Ordering::SeqCst)
            && self.speed.load(Ordering::SeqCst) == other.speed.load(Ordering::SeqCst)
            && *self.phase.lock().expect("") == *other.phase.lock().expect("")
    }
}

impl DownloadState {
    pub fn reset(&self, ordering: Ordering) {
        self.completed_tasks.store(0, ordering);
        self.total_tasks.store(0, ordering);
        self.completed_bytes.store(0, ordering);
        self.total_bytes.store(0, ordering);
        self.speed.store(0, ordering);
    }
}
