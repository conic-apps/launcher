// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;
use tauri::command;
use uuid::Uuid;

use crate::{
    Result,
    microsoft::{
        MicrosoftAccount,
        device_code::{DeviceCodePollResult, DeviceCodeResponse},
    },
};

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
    let (access_token, refresh_token) = crate::microsoft::redeem_access_token(&code).await?;
    Ok(GetAccessTokenResult {
        access_token,
        refresh_token,
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
