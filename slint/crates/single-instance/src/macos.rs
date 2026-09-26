// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Single-instance guard on macOS, over a Unix domain socket.
//!
//! There is no session bus on macOS, and no way to pass a payload through one
//! anyway, so the primary binds a socket and a later launch connects to it,
//! writes its command line and quits. `tauri-plugin-single-instance` does the
//! same; the one difference is the name, which is qualified with the uid so a
//! socket belonging to another user in the shared `/tmp` cannot be taken for
//! ours.
//!
//! The socket is the lock: a launch that can connect to it is a later launch,
//! and one that cannot has to bind it. A socket file left behind by a process
//! that was killed is not a problem, because nothing is listening on it and the
//! connection is refused — the same "nobody is there" answer a missing file
//! gives — so the binding path takes it over. Any other answer to the connection
//! (a full listen backlog, a permission problem) means an instance may well be
//! there after all, so the socket is left alone and the launch runs alongside it
//! rather than taking a live one over.

use std::{
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use crate::framing::{decode, encode};
use crate::{APP_ID, AlreadyRunning, Launch, current_launch, report};

/// How much of a report is read before it is given up on.
///
/// A report is a working directory and a command line, so this is generous by
/// orders of magnitude; it is here so a peer that never stops writing cannot
/// keep the reader busy forever.
const REPORT_LIMIT: u64 = 1 << 20;

/// The socket the primary listens on.
fn socket_path() -> PathBuf {
    // The uid needs `getuid`, which is a plain C call and so an `unsafe` block
    // however harmless it is.
    let uid = unsafe { libc::getuid() };
    let identifier = APP_ID.replace(['.', '-'], "_");
    let mut path = std::env::temp_dir();
    path.push(format!("{identifier}_{uid}.sock"));
    path
}

/// Claims the role by binding the socket, having first made sure nobody else is
/// listening on it.
///
/// # Errors
///
/// [`AlreadyRunning`] once the launch has been handed to the process that is
/// already running.
pub(crate) fn claim(launches: Sender<Launch>) -> Result<Guard, AlreadyRunning> {
    let path = socket_path();

    match notify(&path) {
        Ok(()) => return Err(AlreadyRunning),
        // Nobody answered: either nothing is running, or the file is a leftover
        // from a process that is gone.
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) => {}
        Err(error) => {
            log::warn!(target: "shell", "another instance may be running: {error}");
            return Ok(Guard { path: None });
        }
    }

    // A socket file would make `bind` fail, and the connection above is what
    // established that nothing is listening on it.
    if let Err(error) = std::fs::remove_file(&path)
        && error.kind() != io::ErrorKind::NotFound
    {
        log::warn!(target: "shell", "{} could not be replaced: {error}", path.display());
    }

    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(error) => {
            log::warn!(target: "shell", "single instance is unavailable: {error}");
            return Ok(Guard { path: None });
        }
    };
    log::debug!(target: "shell", "listening on {}", path.display());

    let served = std::thread::Builder::new()
        .name("conic-single-instance".into())
        .spawn(move || serve(listener, launches));
    if served.is_err() {
        // The socket is bound but nothing reads it, so every later launch would
        // hand its report to a queue nobody watches and then quit. Give the
        // socket back and run unguarded instead.
        let _ = std::fs::remove_file(&path);
        return Ok(Guard { path: None });
    }

    Ok(Guard { path: Some(path) })
}

/// Hands this launch to the instance that is already running, if there is one.
fn notify(path: &Path) -> io::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    write!(stream, "{}", encode(&current_launch()))?;
    // The read on the other side runs to the end of the report, which only
    // arrives once this side is done writing.
    stream.shutdown(std::net::Shutdown::Write)
}

/// Accepts reports until the process goes away, handing each to the app.
fn serve(listener: UnixListener, launches: Sender<Launch>) {
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                log::debug!(target: "shell", "a launch was not accepted: {error}");
                continue;
            }
        };
        let mut raw = String::new();
        let read = (&mut stream).take(REPORT_LIMIT).read_to_string(&mut raw);
        match read {
            Ok(_) => {}
            Err(error) => {
                log::debug!(target: "shell", "a launch was not received: {error}");
                continue;
            }
        }
        if let Some(launch) = decode(&raw) {
            report(&launches, launch);
        }
    }
}

/// Holds the bound socket for as long as the primary is running.
pub(crate) struct Guard {
    /// The socket file to take away again, or `None` when there is no socket to
    /// hold and the app is running without the guard.
    path: Option<PathBuf>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        // A launch that follows a crash cleans up after it anyway (see the
        // module comment), so a failure here is not worth reporting.
        let _ = std::fs::remove_file(path);
    }
}
