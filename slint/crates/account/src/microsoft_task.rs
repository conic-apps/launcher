// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Stands in for `crates/account/src/microsoft_commands.rs` — the one command
//! module of the original that carries state of its own.
//!
//! `cmd_spawn_microsoft_login_task` runs at most one login task at a time and
//! `cmd_cancel_microsoft_login_task` aborts it; between them they own the
//! `PluginState { task: Arc<Mutex<Option<AbortHandle>>> }` that the Tauri plugin
//! `manage`s and a command reaches through `State<'_, PluginState>`. With no
//! IPC layer the same state becomes a plain value the app holds and calls:
//! [`LoginTaskState::spawn`] does what the command did — it awaits the task it
//! started, reporting progress through the [`LoginReporter`] as it runs — and
//! [`LoginTaskState::cancel`] is the other command.
//!
//! The original's other two command modules (`offline_commands.rs`,
//! `yggdrasil_commands.rs`) only forwarded their arguments to the module
//! functions, which are `pub` already, so they have no counterpart here.

use std::sync::{Arc, Mutex};

use log::warn;

use crate::{
    Error, Result,
    microsoft::{
        LoginEvent, LoginReporter, MicrosoftAccount, login_with_auth_code, login_with_device_code,
    },
};

/// The at-most-one running Microsoft login task (the original's `PluginState`).
#[derive(Clone, Default)]
pub struct LoginTaskState {
    task: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
}

impl LoginTaskState {
    /// Whether a login task is running right now.
    pub fn is_running(&self) -> bool {
        self.task.lock().expect("Internal error").is_some()
    }

    /// Starts the login task and awaits it, reporting its progress through
    /// `reporter` (`cmd_spawn_microsoft_login_task`).
    ///
    /// `code` is the authorization code of the browser flow; `None` runs the
    /// device-code flow instead. A call while another task is running fails
    /// with [`Error::LoginInProgress`].
    pub async fn spawn(
        &self,
        code: Option<String>,
        reporter: LoginReporter,
    ) -> Result<MicrosoftAccount> {
        {
            let current_task = self.task.lock().expect("Internal error");
            if current_task.is_some() {
                return Err(Error::LoginInProgress);
            }
        }
        let handle = tokio::spawn(async move {
            reporter.report(LoginEvent::Prepare);
            match code {
                Some(code) => login_with_auth_code(&code, &reporter).await,
                None => login_with_device_code(&reporter).await,
            }
        });
        *self.task.lock().expect("Internal error") = Some(handle.abort_handle());
        let result = match handle.await {
            Ok(result) => result,
            Err(error) => {
                warn!("Microsoft login cancelled");
                Err(Error::Aborted(error))
            }
        };
        *self.task.lock().expect("Internal error") = None;
        result
    }

    /// Aborts the running login task (`cmd_cancel_microsoft_login_task`).
    pub fn cancel(&self) {
        let mut current_task = self.task.lock().expect("Internal error");
        if let Some(handle) = current_task.take() {
            handle.abort();
            warn!("Cancelling Microsoft login!");
        }
    }
}
