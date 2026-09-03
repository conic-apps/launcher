// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::{Arc, Mutex};

use log::warn;
use serde::Serialize;
use tauri::{State, command, ipc::Channel};
use uuid::Uuid;

use crate::{
    Error, Result,
    microsoft::{
        LoginEvent, LoginReporter, MicrosoftAccount,
        device_code::{DeviceCodePollResult, DeviceCodeResponse},
        login_with_auth_code, login_with_device_code,
    },
};

/// State of the at-most-one running Microsoft login task.
#[derive(Clone, Default)]
pub(crate) struct PluginState {
    task: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
}

#[command]
pub(crate) async fn cmd_spawn_microsoft_login_task(
    state: State<'_, PluginState>,
    code: Option<String>,
    channel: Channel<LoginEvent>,
) -> Result<MicrosoftAccount> {
    {
        let current_task = state.task.lock().expect("Internal error");
        if current_task.is_some() {
            return Err(Error::LoginInProgress);
        }
    }
    let reporter = LoginReporter::new(channel);
    let handle = tokio::spawn(async move {
        reporter.report(LoginEvent::Prepare);
        match code {
            Some(code) => login_with_auth_code(&code, &reporter).await,
            None => login_with_device_code(&reporter).await,
        }
    });
    *state.task.lock().expect("Internal error") = Some(handle.abort_handle());
    let result = match handle.await {
        Ok(result) => result,
        Err(error) => {
            warn!("Microsoft login cancelled");
            Err(Error::Aborted(error))
        }
    };
    *state.task.lock().expect("Internal error") = None;
    result
}

#[command]
pub(crate) fn cmd_cancel_microsoft_login_task(state: State<'_, PluginState>) {
    let mut current_task = state.task.lock().expect("Internal error");
    if let Some(handle) = current_task.take() {
        handle.abort();
        warn!("Cancelling Microsoft login!");
    }
}

#[command]
pub(crate) async fn cmd_microsoft_get_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    crate::microsoft::get_account(uuid).await
}

#[command]
pub(crate) async fn cmd_microsoft_delete_account(uuid: Uuid) -> Result<()> {
    crate::microsoft::delete_account(uuid).await
}

#[command]
pub(crate) async fn cmd_microsoft_add_account(account: MicrosoftAccount) -> Result<()> {
    crate::microsoft::add_account(account).await
}

#[command]
pub(crate) async fn cmd_microsoft_update_account(
    uuid: Uuid,
    account: MicrosoftAccount,
) -> Result<()> {
    crate::microsoft::update_account(uuid, &account).await
}

#[derive(Serialize)]
pub struct GetAccessTokenResult {
    access_token: String,
    refresh_token: String,
}

#[command]
pub(crate) async fn cmd_microsoft_redeem_access_token(
    code: String,
) -> Result<GetAccessTokenResult> {
    let tokens = crate::microsoft::redeem_access_token(&code).await?;
    Ok(GetAccessTokenResult {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}

#[command]
pub(crate) async fn cmd_microsoft_access_token_auth_flow(
    access_token: String,
    refresh_token: String,
) -> Result<MicrosoftAccount> {
    crate::microsoft::access_token_auth_flow(&access_token, &refresh_token).await
}

#[command]
pub(crate) async fn cmd_microsoft_refresh_account(
    uuid: Uuid,
    force_refresh: bool,
) -> Result<MicrosoftAccount> {
    crate::microsoft::refresh_account(uuid, force_refresh).await
}

#[command]
pub(crate) async fn cmd_microsoft_request_device_code() -> Result<DeviceCodeResponse> {
    crate::microsoft::device_code::request_device_code().await
}

#[command]
pub(crate) async fn cmd_microsoft_poll_device_code(
    device_code: String,
) -> Result<DeviceCodePollResult> {
    crate::microsoft::device_code::poll_device_code(&device_code).await
}
