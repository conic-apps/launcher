// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The at-most-one running Microsoft login task.
//!
//! [`LoginTaskState::spawn`] starts the flow and awaits it, reporting progress
//! through the [`LoginReporter`]; a call while one is already running fails
//! with [`Error::LoginInProgress`]. [`LoginTaskState::cancel`] aborts it.

use std::sync::{Arc, Mutex};

use log::warn;

use crate::{
    Error, Result,
    microsoft::{
        LoginEvent, LoginReporter, MicrosoftAccount, login_with_auth_code, login_with_device_code,
    },
};

/// Which of the two Microsoft flows a login task runs.
///
/// The browser flow carries two inseparable values: the authorization code and
/// the `redirect_uri` it was issued against, which has to be repeated exactly
/// at the token endpoint. They travel together so neither can be passed
/// without the other.
#[derive(Clone, Debug)]
pub enum LoginRequest {
    /// The device-code flow: no browser round trip, nothing to wait for but the
    /// user's own code entry.
    DeviceCode,
    /// The browser flow's authorization code, and the `redirect_uri` the browser
    /// came back to — a loopback listener this app owns (`authcode`), so the
    /// port is chosen when the flow starts and is not known before that.
    AuthCode { code: String, redirect_uri: String },
}

/// The at-most-one running Microsoft login task.
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
    /// `reporter`.
    ///
    /// A call while another task is running fails with
    /// [`Error::LoginInProgress`].
    pub async fn spawn(
        &self,
        request: LoginRequest,
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
            match request {
                LoginRequest::AuthCode { code, redirect_uri } => {
                    login_with_auth_code(&code, &redirect_uri, &reporter).await
                }
                LoginRequest::DeviceCode => login_with_device_code(&reporter).await,
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

    /// Aborts the running login task.
    pub fn cancel(&self) {
        let mut current_task = self.task.lock().expect("Internal error");
        if let Some(handle) = current_task.take() {
            handle.abort();
            warn!("Cancelling Microsoft login!");
        }
    }
}
