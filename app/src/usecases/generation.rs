// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A monotonic guard for one logical run or request: the use case issues a
//! [`Token`] and invalidates it, the frontend checks it on the event loop before
//! applying the result.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// One logical request's claim on the interface, checked before its result is
/// applied: a newer token, or an invalidated gate, makes every older token stop
/// being current.
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
