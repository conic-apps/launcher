// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

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
pub(crate) async fn cmd_yggdrasil_get_server_info(api_root: String) -> Result<YggdrasilServerInfo> {
    crate::yggdrasil::yggdrasil_server::get_server_info(&api_root).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_authenticate_account(
    api_root: String,
    username: String,
    password: String,
) -> Result<AuthResponse> {
    crate::yggdrasil::yggdrasil_user_api::authenticate(&api_root, username, password).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_validate_account(account: YggdrasilAccount) -> Result<bool> {
    crate::yggdrasil::yggdrasil_user_api::validate(account).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_refresh_account(
    account: YggdrasilAccount,
) -> Result<YggdrasilAccount> {
    crate::yggdrasil::yggdrasil_user_api::refresh(account).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_invalidate_account(
    api_root: String,
    access_token: String,
    client_token: String,
) -> Result<()> {
    crate::yggdrasil::yggdrasil_user_api::invalidate(&api_root, access_token, client_token).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_get_profile(api_root: &str, uuid: Uuid) -> Result<Profile> {
    crate::yggdrasil::yggdrasil_user_api::get_profile(api_root, uuid).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_get_profiles(
    api_root: String,
    uuids: Vec<Uuid>,
) -> Result<Vec<Profile>> {
    crate::yggdrasil::yggdrasil_user_api::get_profiles(&api_root, uuids).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_add_account(account: YggdrasilAccount) -> Result<()> {
    crate::yggdrasil::add_account(account).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_delete_account(account: YggdrasilAccount) -> Result<()> {
    crate::yggdrasil::delete_account(account).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_get_account(account_key: Uuid) -> Result<YggdrasilAccount> {
    crate::yggdrasil::get_account(account_key).await
}

#[command]
pub(crate) async fn cmd_yggdrasil_list_accounts() -> Result<Vec<YggdrasilAccount>> {
    crate::yggdrasil::list_accounts().await
}

#[command]
pub(crate) async fn cmd_yggdrasil_update_account(
    account_key: Uuid,
    account: YggdrasilAccount,
) -> Result<()> {
    crate::yggdrasil::update_account(account_key, account).await
}
