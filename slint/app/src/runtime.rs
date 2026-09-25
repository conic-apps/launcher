// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The tokio runtime the app's background work runs on.
//!
//! The Tauri app gets one from Tauri itself: its commands, `tauri::async_runtime`
//! and the async plugins all run on the runtime Tauri builds at startup, on a
//! pool of worker threads. The Slint app owns the event loop itself, so it
//! builds the same thing — a multi-threaded tokio runtime — and hands out the
//! two entry points that stand in for Tauri's:
//!
//!   * [`spawn`] for the async work (the HTTP calls of `slint-install`), and
//!   * [`spawn_blocking`] for the work that would otherwise sit on a runtime
//!     thread (the disk scans, `tauri::async_runtime::spawn_blocking`).
//!
//! Neither may touch the UI: Slint is not thread-safe. A task reports back with
//! `Weak::upgrade_in_event_loop` — see `create_instance.rs`.

use std::future::Future;

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

/// The runtime, built on first use. It is never dropped, like Tauri's.
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("conic-worker")
        .build()
        .expect("failed to build the tokio runtime")
});

/// Runs `future` on the runtime (`tauri::async_runtime::spawn`).
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.spawn(future)
}

/// Runs `task` on the runtime's blocking pool
/// (`tauri::async_runtime::spawn_blocking`).
pub fn spawn_blocking<F, R>(task: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn_blocking(task)
}
