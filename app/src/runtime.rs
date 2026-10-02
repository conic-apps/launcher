// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The tokio runtime the app's background work runs on.
//!
//! The app owns the event loop itself, so it builds a multi-threaded tokio
//! runtime and hands out the entry points the background work uses:
//!
//!   * [`spawn`] for the async work (the HTTP calls of `install`), and
//!   * [`spawn_blocking`] for work that would otherwise sit on a runtime thread
//!     (the disk scans).
//!
//! Neither may touch the UI: Slint is not thread-safe. A task reports back with
//! `Weak::upgrade_in_event_loop` — see `create_instance.rs`.

use std::future::Future;

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

/// The runtime, built on first use. It is never dropped.
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("conic-worker")
        .build()
        .expect("failed to build the tokio runtime")
});

/// Runs `future` on the runtime.
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.spawn(future)
}

/// Runs `task` on the runtime's blocking pool.
pub fn spawn_blocking<F, R>(task: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn_blocking(task)
}

/// Runs `future` to completion on the calling thread.
///
/// For the handful of helpers that block inside an async task — the content
/// cards' icons, which are fetched over the network while a list is being
/// built, and the synchronous reads the `content` crate offers. `block_in_place`
/// hands this worker's other tasks to the pool first, so the call parks one
/// thread rather than the whole runtime. It needs the multi-threaded runtime,
/// which is what this module builds.
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
