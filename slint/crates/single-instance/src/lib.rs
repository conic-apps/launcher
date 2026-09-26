// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Single-instance guard for the Slint app.
//!
//! The Tauri app gets this from `tauri-plugin-single-instance`: it claims a
//! platform-wide lock while it starts, and any later launch of the app finds
//! the lock, hands its command line to the process that is already running and
//! exits — so the user gets the window they just asked for rather than a second
//! copy of the launcher. This crate is the same mechanism without Tauri: the
//! Slint app owns its event loop, so it claims the lock itself and is told
//! about later launches through a channel.
//!
//! One backend per platform, all behind the same two types:
//!
//! | Platform | Lock                     | Later launch is reported by   |
//! | -------- | ------------------------ | ----------------------------- |
//! | Linux    | a D-Bus well-known name  | an `ExecuteCallback` method   |
//! | macOS    | a `UDS` socket file      | a connection to that socket  |
//! | Windows  | a named mutex           | a `WM_COPYDATA` message       |
//!
//! Any other platform has no backend and always claims the role, so the app
//! still starts there — a case that does not arise today, since
//! `slint-platform` only reports Windows, Linux and macOS.
//!
//! # Use
//!
//! ```no_run
//! let Ok(single_instance) = slint_single_instance::try_acquire() else {
//!     // Another instance is running and has just been told about this launch.
//!     return;
//! };
//! ```
//!
//! and the reports arrive through [`SingleInstance::next_launch`], on whichever
//! thread the backend uses — a D-Bus worker, a window message, a socket reader.
//! Anything that touches the UI has to hop to the event loop from there.

use std::sync::mpsc::{Receiver, Sender, channel};

/// Unused on Linux, where the bus carries the two fields as method arguments.
mod framing;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// The identity the platform locks are derived from.
///
/// The Tauri app's identifier is `app.conicmc.launcher` (`core/tauri.conf.json`);
/// the Slint app takes `.slint` so the two frontends can be run side by side
/// while both exist — sharing the identifier would mean one of them exits
/// immediately in favour of the other. Drop the suffix once the Slint app
/// replaces the Tauri one, so the desktop entry's `conic-launcher://` handler
/// reaches whichever build is installed.
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

/// A later launch of the app, as the running instance is told about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The later process' command line, `argv[0]` included.
    pub args: Vec<String>,

    /// The working directory it was started in.
    pub cwd: String,
}

/// The app is already running, and has been told about this launch.
///
/// The caller has nothing left to do and should exit without starting anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlreadyRunning;

/// Claims the single-instance role for this process, if it is free.
///
/// The returned [`SingleInstance`] holds the platform lock for as long as it is
/// alive, so it has to be kept for the lifetime of the app: dropping it lets
/// another launch in. It also reports the later launches that are handed to it.
///
/// This is meant to run before anything else is set up — before the event
/// loop, before a window, before reading the config — so a second launch quits
/// without ever showing a window of its own.
///
/// # Errors
///
/// [`AlreadyRunning`] when the role was taken: the running instance has been
/// told about this launch, and the caller should exit.
///
/// A backend that cannot claim the role for an unrelated reason (no D-Bus
/// session, no session to hand the report to) logs a warning and lets the
/// launch continue, so a broken environment cannot keep the app from starting.
pub fn try_acquire() -> Result<SingleInstance, AlreadyRunning> {
    let (launches, queue) = channel();
    let _guard = backend::claim(launches)?;
    Ok(SingleInstance { _guard, queue })
}

/// The claim on the single-instance role, and the later launches handed to it.
pub struct SingleInstance {
    /// Keeps the platform lock (a D-Bus name, a bound socket, a mutex handle
    /// and its message window) alive for as long as this value lives. It is
    /// never read; dropping it is the point.
    _guard: backend::Guard,

    /// Filled by the backend from whichever thread it receives a launch on.
    queue: Receiver<Launch>,
}

impl SingleInstance {
    /// Waits for the next launch of the app and returns what it carried.
    ///
    /// Returns `None` once the claim is gone — the app is shutting down — so
    /// this is the whole loop of whoever watches for later launches.
    pub fn next_launch(&self) -> Option<Launch> {
        self.queue.recv().ok()
    }
}

/// Collects this process' command line and working directory for a launch that
/// has to be handed to the instance that is already running.
pub(crate) fn current_launch() -> Launch {
    Launch {
        args: std::env::args().collect(),
        cwd: std::env::current_dir()
            .ok()
            .and_then(|directory| directory.into_os_string().into_string().ok())
            .unwrap_or_default(),
    }
}

/// A backend reports a launch it cannot hand over (the process holding the lock
/// is gone, the socket has no listener) by returning an error; there is nothing
/// the caller can do about it beyond the log line, which keeps the noise down.
pub(crate) fn report(launches: &Sender<Launch>, launch: Launch) {
    if let Err(error) = launches.send(launch) {
        log::debug!(target: "shell", "a launch was not handed over: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::SingleInstance;

    /// The claim has to survive a move onto a thread, because that is where the
    /// app waits for later launches (it cannot be told about them before its
    /// event loop exists). Nothing inside the crate requires that, so nothing
    /// inside the crate would notice it breaking either: on Windows the claim
    /// holds a `HANDLE` and an `HWND`, which are `*mut c_void` and therefore
    /// `!Send` until the backend says otherwise.
    #[test]
    fn the_claim_can_move_to_another_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<SingleInstance>();
    }
}
