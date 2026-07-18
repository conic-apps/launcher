// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use tauri::command;
use uuid::Uuid;

use crate::{
    Result,
    yggdrasil::{
        YggdrasilAccount,
        yggdrasil_server::YggdrasilServerInfo,
        yggdrasil_user_api::{AuthResponse, Profile},
    },
};

#[command]
pub async fn cmd_add_yggdrasil_server(api_root: String) -> Result<()> {
    crate::yggdrasil::yggdrasil_server::add(&api_root).await
}

#[command]
pub async fn cmd_delete_yggdrasil_server(index_to_delete: usize) -> Result<()> {
    crate::yggdrasil::yggdrasil_server::delete(index_to_delete).await
}

#[command]
pub async fn cmd_list_yggdrasil_server() -> Result<Vec<String>> {
    crate::yggdrasil::yggdrasil_server::list_all().await
}

#[command]
pub async fn cmd_get_yggdrasil_server_info(api_root: String) -> Result<YggdrasilServerInfo> {
    crate::yggdrasil::yggdrasil_server::get_server_info(&api_root).await
}

#[command]
pub async fn cmd_yggdrasil_authenticate_account(
    api_root: String,
    username: String,
    password: String,
) -> Result<AuthResponse> {
    crate::yggdrasil::yggdrasil_user_api::authenticate(&api_root, username, password).await
}

#[command]
pub async fn cmd_yggdrasil_validate_account(account: YggdrasilAccount) -> Result<bool> {
    crate::yggdrasil::yggdrasil_user_api::is_account_token_valid(account).await
}

#[command]
pub async fn cmd_yggdrasil_refresh_account(account: YggdrasilAccount) -> Result<YggdrasilAccount> {
    crate::yggdrasil::yggdrasil_user_api::refresh(account).await
}

#[command]
pub async fn cmd_yggdrasil_invalidate_account(
    api_root: String,
    access_token: String,
    client_token: String,
) -> Result<()> {
    crate::yggdrasil::yggdrasil_user_api::invalidate(&api_root, access_token, client_token).await
}

#[command]
pub async fn cmd_yggdrasil_get_profile(api_root: &str, uuid: Uuid) -> Result<Profile> {
    crate::yggdrasil::yggdrasil_user_api::get_profile(api_root, uuid).await
}

#[command]
pub async fn cmd_add_yggdrasil_account(account: YggdrasilAccount) -> Result<()> {
    crate::yggdrasil::add_account(account).await
}

#[command]
pub async fn cmd_delete_yggdrasil_account(account_key: Uuid) -> Result<()> {
    crate::yggdrasil::delete_account(account_key).await
}

#[command]
pub async fn cmd_get_yggdrasil_account(account_key: Uuid) -> Result<YggdrasilAccount> {
    crate::yggdrasil::get_account(account_key).await
}

#[command]
pub async fn cmd_list_yggdrasil_accounts() -> Result<HashMap<Uuid, YggdrasilAccount>> {
    crate::yggdrasil::list_accounts().await
}

#[command]
pub async fn cmd_update_yggdrasil_account(
    account_key: Uuid,
    account: YggdrasilAccount,
) -> Result<()> {
    crate::yggdrasil::update_account(account_key, account).await
}
