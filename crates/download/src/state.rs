// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum DownloadPhase {
    VerifyExistingFiles,
    DownloadFiles,
    VerifyResult,
    Unknown,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AtomicDownloadState {
    pub completed: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
    pub phase: Arc<Mutex<DownloadPhase>>,
    pub speed: Arc<AtomicU64>,
}

impl Default for AtomicDownloadState {
    fn default() -> Self {
        Self {
            completed: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
            speed: Arc::new(AtomicU64::new(0)),
            phase: Arc::new(Mutex::new(DownloadPhase::Unknown)),
        }
    }
}

pub trait DownloadState {
    fn reset(&self);
    fn set_total(&self, value: u64);
    fn set_completed(&self, value: u64);
    fn set_speed(&self, value: u64);
    fn set_phase(&self, phase: DownloadPhase);
    fn fetch_add_completed(&self, value: u64) -> u64;
}

impl DownloadState for AtomicDownloadState {
    fn reset(&self) {
        self.completed.store(0, Ordering::SeqCst);
        self.total.store(0, Ordering::SeqCst);
        self.speed.store(0, Ordering::SeqCst);
    }

    fn set_total(&self, value: u64) {
        self.total.store(value, Ordering::SeqCst);
    }

    fn set_completed(&self, value: u64) {
        self.completed.store(value, Ordering::SeqCst);
    }

    fn set_speed(&self, value: u64) {
        self.speed.store(value, Ordering::SeqCst);
    }

    fn set_phase(&self, phase: DownloadPhase) {
        let mut self_phase = self.phase.lock().expect("Internal error");
        *self_phase = phase;
    }

    fn fetch_add_completed(&self, value: u64) -> u64 {
        self.completed.fetch_add(value, Ordering::SeqCst)
    }
}
