// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum DownloadPhase {
    VerifyExistingFiles,
    #[default]
    DownloadFiles,
}

/// A plain reading of a [`DownloadState`], safe to cross a thread.
///
/// `DownloadState` cannot be compared or moved around usefully on its own: the
/// counters sit behind `Arc`s, so a clone shares them and always equals the
/// original. This is the value a use case samples and reports through its
/// [`shared::Sink`], and `PartialEq` on it is what lets a reporter drop the
/// ticks where nothing moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DownloadSnapshot {
    pub phase: DownloadPhase,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
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

    pub fn snapshot(&self) -> DownloadSnapshot {
        DownloadSnapshot {
            phase: *self.phase.lock().expect("Internal error"),
            completed_bytes: self.completed_bytes.load(Ordering::SeqCst),
            total_bytes: self.total_bytes.load(Ordering::SeqCst),
        }
    }
}

/// How often [`watch`] samples a running download.
pub const WATCH_INTERVAL: Duration = Duration::from_millis(100);

/// Runs `work` while reporting `state` on a clock, then runs `work` to its
/// result and reports once more.
///
/// This is the sampler every use case that downloads reports through: the
/// download itself only updates the shared counters (cheap, lock-free, and free
/// to run from several tasks at once), and the sampling and the change test live
/// here rather than in the interface layer. `report` is called with the current
/// snapshot immediately and then every [`WATCH_INTERVAL`], and once more after
/// `work` resolves so the final byte count is never missed.
pub async fn watch<F, R>(state: &DownloadState, mut report: R, work: F) -> F::Output
where
    F: Future,
    R: FnMut(DownloadSnapshot),
{
    tokio::pin!(work);
    let mut ticker = tokio::time::interval(WATCH_INTERVAL);
    // A long step between ticks must not produce a burst of catch-up ticks.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            output = &mut work => {
                report(state.snapshot());
                return output;
            }
            // The first tick fires immediately, which is the initial report.
            _ = ticker.tick() => report(state.snapshot()),
        }
    }
}
