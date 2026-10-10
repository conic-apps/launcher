// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A monotonic guard for one logical run or request.
//!
//! This is a use-case concern, not a Slint one: a run is superseded when a newer
//! one starts, and a result that lands afterwards must be dropped. The frontend
//! checks the [`Token`] on the event loop; the use case only issues and
//! invalidates it, so nothing here mentions Slint.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// One logical request's claim on the interface.
///
/// A token is issued by a [`Gate`] and checked before its result is applied.
/// When the gate issues a newer token — or is invalidated — every older token
/// stops being current, so a slow answer that comes back after a newer request
/// cannot overwrite it.
#[derive(Clone)]
pub(crate) struct Token {
    generation: Arc<AtomicU64>,
    mine: u64,
}

impl Token {
    /// Whether this request is still the newest one.
    pub(crate) fn is_current(&self) -> bool {
        self.generation.load(Ordering::SeqCst) == self.mine
    }
}

/// Issues [`Token`]s for one logical slot.
///
/// One gate per slot — a list, a detail panel, a launch flow — so a newer
/// request in one slot does not cancel an unrelated one elsewhere.
#[derive(Clone, Default)]
pub(crate) struct Gate {
    generation: Arc<AtomicU64>,
}

impl Gate {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Starts a new request, invalidating every earlier token.
    pub(crate) fn issue(&self) -> Token {
        let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        Token {
            generation: Arc::clone(&self.generation),
            mine,
        }
    }

    /// Invalidates the outstanding token, so nothing in flight is delivered.
    pub(crate) fn invalidate(&self) {
        // Reached by the launch view's cancel button and the multiplayer
        // download's cancel. Without this, "launch flow started" and then
        // nothing is the whole trace of a launch the user gave up on.
        log::debug!(target: "shell", "the outstanding request was cancelled; its result will \
             be dropped");
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}
