// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The tokio runtime for background work. Neither entry point may touch the UI;
//! tasks report back through `crate::ui::services::report`.
//!
//! Spawned tasks are wrapped so a panic reaches the log: the call sites drop
//! their handles, and a detached panic would otherwise only reach stderr.

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

/// Runs `future` on the runtime. A panic is logged, then re-raised so the handle
/// still resolves to a `JoinError`.
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
                std::panic::resume_unwind(panic)
            }
        }
    })
}

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

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> &str {
    panic
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<a non-string panic payload>")
}

/// Runs `future` to completion on the calling thread. `block_in_place` parks
/// one worker rather than the whole runtime, so the multi-threaded runtime is
/// required.
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
