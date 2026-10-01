// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Single-instance guard on Linux, over the session bus.
//!
//! The primary instance owns the well-known name
//! `app.conicmc.launcher.slint.SingleInstance` and serves
//! `/app/conicmc/launcher/slint/SingleInstance`; a later launch finds the name
//! taken, calls the primary's `ExecuteCallback` with its own command line and
//! quits. This is what `tauri-plugin-single-instance` does on Linux, and it is
//! the only mechanism that carries a later launch's arguments: the bus is the
//! one thing both processes can agree on without a shared file.

use std::sync::mpsc::Sender;

use zbus::{blocking::Connection, interface, names::WellKnownName};

use crate::{APP_ID, AlreadyRunning, Launch, current_launch, report};

/// The interface the primary serves, kept as the Tauri plugin has it so a
/// secondary of one frontend still reaches a primary of the other.
const INTERFACE: &str = "org.SingleInstance.DBus";

/// The service the primary serves the launch report on.
fn object_path() -> String {
    format!("/{}", APP_ID.replace('.', "/").replace('-', "_"))
}

/// The well-known name the primary owns; owning it *is* being the primary.
fn bus_name() -> String {
    format!("{APP_ID}.SingleInstance")
}

/// The object behind [`INTERFACE`], which is how a later launch reaches the
/// running instance.
struct Server {
    /// Where a reported launch goes. The bus dispatches the call on one of its
    /// own threads, so nothing here may touch the UI.
    launches: Sender<Launch>,
}

#[interface(name = "org.SingleInstance.DBus")]
impl Server {
    /// Hands a later launch's command line to the running instance.
    fn execute_callback(&mut self, argv: Vec<String>, cwd: String) {
        report(&self.launches, Launch { args: argv, cwd });
    }
}

/// Claims the role by requesting [`bus_name`].
///
/// # Errors
///
/// [`AlreadyRunning`] once the name has been handed to the instance that owns
/// it — the launch is delivered first, so the caller can quit right after.
pub(crate) fn claim(launches: Sender<Launch>) -> Result<Guard, AlreadyRunning> {
    let name = bus_name();
    let path = object_path();

    let builder = match zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.name(name.as_str()))
        .and_then(|builder| builder.serve_at(path.as_str(), Server { launches }))
    {
        Ok(builder) => builder
            .replace_existing_names(false)
            .allow_name_replacements(false),
        Err(error) => {
            // No session bus (a bare tty, a container without one): there is
            // nothing to claim a name on, so start the app rather than refuse
            // to.
            log::warn!(target: "shell", "single instance is unavailable: {error}");
            return Ok(Guard::unclaimed());
        }
    };

    match builder.build() {
        Ok(connection) => Ok(Guard {
            connection: Some(connection),
            name,
        }),
        Err(zbus::Error::NameTaken) => {
            let launch = current_launch();
            if let Err(error) = Connection::session().and_then(|connection| {
                connection.call_method(
                    Some(name.as_str()),
                    path.as_str(),
                    Some(INTERFACE),
                    "ExecuteCallback",
                    &(launch.args.clone(), launch.cwd.clone()),
                )
            }) {
                // The name is taken but nothing answered on it. The object
                // server is registered before the name is requested, and the
                // bus takes the name away as soon as the connection owning it
                // goes, so this is not a primary that is still starting up —
                // it is a name something else has taken. There is nothing to
                // take it back from, and quitting is what the Tauri app's
                // plugin does in the same situation.
                log::warn!(target: "shell", "the running instance did not answer: {error}");
            }
            Err(AlreadyRunning)
        }
        Err(error) => {
            log::warn!(target: "shell", "single instance is unavailable: {error}");
            Ok(Guard::unclaimed())
        }
    }
}

/// The claim: the bus connection that owns [`bus_name`].
///
/// The name is released when the connection is dropped, so this is also what
/// lets the next launch in.
pub(crate) struct Guard {
    /// `None` when there was no session bus to claim a name on, which leaves
    /// the app running without the guard.
    connection: Option<Connection>,

    /// The name being held, kept for the log line when it is given up.
    name: String,
}

impl Guard {
    /// A guard for a launch that could not claim the name.
    fn unclaimed() -> Self {
        Self {
            connection: None,
            name: String::new(),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(connection) = self.connection.as_ref() else {
            return;
        };
        // An owned name goes back to the bus as the connection drops anyway;
        // releasing it explicitly makes the moment it is given up observable
        // (and it is the name, not the connection, that the next launch races
        // for).
        let Ok(name) = WellKnownName::try_from(self.name.clone()) else {
            return;
        };
        if let Err(error) = connection.release_name(name) {
            log::debug!(target: "shell", "the single-instance name was not released: {error}");
        }
    }
}
