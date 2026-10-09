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
//! `crate::ui::services::report::report` — see `create_instance.rs`.
//!
//! Every spawned task is wrapped so that a panic in it reaches the log before it
//! reaches the process. Nothing awaited the handles: the ~30 call sites drop them,
//! and tokio *detaches* on a drop rather than aborting, so the work still ran to
//! completion — but a panicking task was previously only ever visible through the
//! default panic hook on stderr, which a GUI launch has nowhere to show. See
//! [`spawn`].

use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
};

use futures::FutureExt;
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
///
/// The returned handle carries the task's real output, and a caller that awaits it
/// still sees a panic as the `JoinError` it would otherwise have seen. What the
/// wrapper adds is the log line: a panicking background task is otherwise the one
/// failure mode this app cannot report, since nothing in it is awaited and a GUI
/// launch shows nobody its stderr.
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.spawn(async move {
        match AssertUnwindSafe(future).catch_unwind().await {
            Ok(output) => output,
            Err(panic) => {
                log::error!("a background task panicked: {}", panic_message(&panic));
                // Re-raised rather than swallowed, so the handle still resolves to
                // a `JoinError` and the default hook still prints the backtrace.
                std::panic::resume_unwind(panic)
            }
        }
    })
}

/// Runs `task` on the runtime's blocking pool.
///
/// Wrapped the same way [`spawn`] is; see it for why.
pub fn spawn_blocking<F, R>(task: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn_blocking(move || match catch_unwind(AssertUnwindSafe(task)) {
        Ok(output) => output,
        Err(panic) => {
            log::error!(
                "a background blocking task panicked: {}",
                panic_message(&panic)
            );
            std::panic::resume_unwind(panic)
        }
    })
}

/// Renders whatever a panic payload turned out to be.
///
/// `Box<dyn Any>` is either the `&'static str` or the `String` the default hook
/// formats, or something else entirely if a panic crossed a C boundary — so the
/// fallback is a placeholder rather than a `Debug` that would not compile.
fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> &str {
    panic
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<a non-string panic payload>")
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
