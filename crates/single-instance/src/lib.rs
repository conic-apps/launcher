// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Single-instance guard.
//!
//! The first launch claims a platform-wide lock; a later launch hands its
//! command line to the running process and exits.
//!
//! | Platform | Lock                    | Later launch reported by    |
//! | -------- | ----------------------- | --------------------------- |
//! | Linux    | a D-Bus well-known name | an `ExecuteCallback` method |
//! | macOS    | a UDS socket file       | a connection to that socket |
//! | Windows  | a named mutex           | a `WM_COPYDATA` message     |
//!
//! Reports arrive through [`SingleInstance::next_launch`] on whichever thread
//! the backend uses, so UI work has to hop to the event loop.

use std::sync::mpsc::{Receiver, Sender, channel};

mod framing;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// The platform locks are derived from this. It matches the macOS bundle
/// identifier.
pub const APP_ID: &str = "app.conicmc.launcher.slint";

#[cfg(target_os = "linux")]
use linux as backend;
#[cfg(target_os = "macos")]
use macos as backend;
#[cfg(target_os = "windows")]
use windows as backend;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use unsupported as backend;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlreadyRunning;

/// Claims the single-instance role. Keep the returned value for the app's
/// lifetime: dropping it releases the lock.
///
/// Returns [`AlreadyRunning`] when another instance has the role. A backend
/// failure that is not contention (no D-Bus session, no session to hand the
/// report to) logs and lets the launch continue, so the app still starts.
pub fn try_acquire() -> Result<SingleInstance, AlreadyRunning> {
    let (launches, queue) = channel();
    let _guard = backend::claim(launches)?;
    Ok(SingleInstance { _guard, queue })
}

pub struct SingleInstance {
    /// Holds the platform lock; never read, dropping it is the point.
    _guard: backend::Guard,
    queue: Receiver<Launch>,
}

impl SingleInstance {
    /// Waits for the next launch, or returns `None` once the claim is gone.
    pub fn next_launch(&self) -> Option<Launch> {
        let launch = self.queue.recv().ok();
        if launch.is_none() {
            log::debug!(target: "shell", "the single-instance feed closed; the claim is gone");
        }
        launch
    }
}

pub(crate) fn current_launch() -> Launch {
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|directory| directory.into_os_string().into_string().ok());
    // A non-UTF-8 directory is handed over as an empty string.
    if cwd.is_none() {
        log::debug!(target: "shell", "the working directory could not be read as text; the \
             launch is handed over without one");
    }
    Launch {
        args: std::env::args().collect(),
        cwd: cwd.unwrap_or_default(),
    }
}

pub(crate) fn report(launches: &Sender<Launch>, launch: Launch) {
    if let Err(error) = launches.send(launch) {
        log::debug!(target: "shell", "a launch was not handed over: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::SingleInstance;

    /// The app waits for later launches on a worker thread. On Windows the
    /// guard holds `HANDLE`/`HWND` pointers, which are `!Send` unless the backend
    /// opts in, and nothing else in the crate would catch that regression.
    #[test]
    fn the_claim_can_move_to_another_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<SingleInstance>();
    }
}
