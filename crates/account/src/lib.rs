// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;

use crate::{
    microsoft::{
        MicrosoftAccount,
        device_code::{DeviceCodePollResult, DeviceCodeResponse},
    },
    offline::OfflineAccount,
    yggdrasil::{
        YggdrasilAccount, yggdrasil_server::YggdrasilServerInfo, yggdrasil_user_api::AuthResponse,
    },
};
pub use error::*;

mod error;
pub mod microsoft;
pub mod offline;
pub mod yggdrasil;

#[derive(Clone, Serialize, Deserialize)]
pub enum Account {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
    Yggdrasil(YggdrasilAccount),
}

impl Account {
    pub fn get_profile_name(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.profile_name.to_string(),
            Account::Yggdrasil(account) => account.profile_name.to_string(),
            Account::Offline(account) => account.name.to_string(),
        }
    }

    pub fn get_profile_uuid(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.uuid.to_string(),
            Account::Yggdrasil(account) => account.profile_uuid.to_string(),
            Account::Offline(account) => account.uuid.to_string(),
        }
    }

    pub fn get_access_token(&self) -> String {
        match self {
            Account::Microsoft(account) => account.minecraft_access_token.to_string(),
            Account::Yggdrasil(account) => account.access_token.to_string(),
            Account::Offline(account) => "114514".to_string(),
        }
    }

    pub fn get_user_type(&self) -> String {
        match self {
            Account::Microsoft(_) => "msa".to_string(),
            Account::Yggdrasil(_) => "mojang".to_string(),
            Account::Offline(_) => "mojang".to_string(),
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("account")
        .invoke_handler(tauri::generate_handler![])
        .build()
}

#[derive(Serialize, Deserialize)]
struct Accounts {
    microsoft: Vec<MicrosoftAccount>,
    offline: Vec<OfflineAccount>,
    third_party_yggdrasil: HashMap<Uuid, YggdrasilAccount>,
}

#[command]
async fn cmd_list_accounts() -> Result<Accounts> {
    Ok(Accounts {
        microsoft: microsoft::list_accounts().await?,
        offline: offline::list_accounts()?,
        third_party_yggdrasil: yggdrasil::list_accounts().await?,
    })
}

#[command]
async fn cmd_get_microsoft_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    microsoft::get_account(uuid).await
}

#[command]
async fn cmd_delete_microsoft_account(uuid: Uuid) -> Result<()> {
    microsoft::delete_account(uuid).await
}

#[command]
async fn cmd_add_microsoft_account(account: MicrosoftAccount) -> Result<()> {
    microsoft::add_account(account).await
}

#[command]
async fn cmd_update_microsoft_account(uuid: Uuid, account: MicrosoftAccount) -> Result<()> {
    microsoft::update_account(uuid, &account).await
}

#[derive(Serialize)]
struct GetAccessTokenResult {
    access_token: String,
    refresh_token: String,
}

#[command]
async fn cmd_get_access_token(code: String) -> Result<GetAccessTokenResult> {
    let (access_token, refresh_token) = microsoft::get_access_token(&code).await?;
    Ok(GetAccessTokenResult {
        access_token,
        refresh_token,
    })
}

#[command]
async fn cmd_microsoft_access_token_auth_flow(
    access_token: String,
    refresh_token: String,
) -> Result<MicrosoftAccount> {
    microsoft::access_token_auth_flow(&access_token, &refresh_token).await
}

#[command]
async fn cmd_refresh_microsoft_account(
    uuid: Uuid,
    force_refresh: bool,
) -> Result<MicrosoftAccount> {
    microsoft::refresh_account(uuid, force_refresh).await
}

#[command]
async fn cmd_request_device_code() -> Result<DeviceCodeResponse> {
    microsoft::device_code::request_device_code().await
}

#[command]
async fn cmd_poll_device_code(device_code: String) -> Result<DeviceCodePollResult> {
    microsoft::device_code::poll_device_code(&device_code).await
}

#[command]
fn cmd_add_offline_account(name: String) -> Result<()> {
    offline::add_account(&name)
}

#[command]
fn cmd_delete_offline_account(uuid: Uuid) -> Result<()> {
    offline::delete_account(uuid)
}

#[command]
fn cmd_update_offline_account(account: OfflineAccount) -> Result<()> {
    offline::update_account(account)
}

#[command]
fn cmd_get_offline_account(uuid: Uuid) -> Result<OfflineAccount> {
    offline::get_account(uuid)
}

#[command]
async fn cmd_add_yggdrasil_server(api_root: String) -> Result<()> {
    yggdrasil::yggdrasil_server::add(&api_root).await
}

#[command]
async fn cmd_delete_yggdrasil_server(index_to_delete: usize) -> Result<()> {
    yggdrasil::yggdrasil_server::delete(index_to_delete).await
}

#[command]
async fn cmd_list_yggdrasil_server() -> Result<Vec<String>> {
    yggdrasil::yggdrasil_server::list_all().await
}

#[command]
async fn cmd_get_yggdrasil_server_info(api_root: String) -> Result<YggdrasilServerInfo> {
    yggdrasil::yggdrasil_server::get_server_info(&api_root).await
}

#[command]
async fn cmd_yggdrasil_authenticate_account(
    api_root: String,
    username: String,
    password: String,
) -> Result<AuthResponse> {
    yggdrasil::yggdrasil_user_api::authenticate(&api_root, username, password).await
}

#[command]
async fn cmd_yggdrasil_validate_account(account: YggdrasilAccount) -> Result<bool> {
    yggdrasil::yggdrasil_user_api::is_account_token_valid(account).await
}

#[command]
async fn cmd_yggdrasil_refresh_account(account: YggdrasilAccount) -> Result<YggdrasilAccount> {
    yggdrasil::yggdrasil_user_api::refresh(account).await
}

#[command]
async fn cmd_yggdrasil_invalidate_account(
    api_root: String,
    access_token: String,
    client_token: String,
) -> Result<()> {
    yggdrasil::yggdrasil_user_api::invalidate(&api_root, access_token, client_token).await
}

#[command]
async fn cmd_yggdrasil_get_profile(
    api_root: &str,
    uuid: Uuid,
) -> Result<yggdrasil::yggdrasil_user_api::Profile> {
    yggdrasil::yggdrasil_user_api::get_profile(api_root, uuid).await
}

#[command]
async fn cmd_add_yggdrasil_account(account: YggdrasilAccount) -> Result<()> {
    yggdrasil::add_account(account).await
}

#[command]
async fn cmd_delete_yggdrasil_account(account_key: Uuid) -> Result<()> {
    yggdrasil::delete_account(account_key).await
}

#[command]
async fn cmd_get_yggdrasil_account(account_key: Uuid) -> Result<YggdrasilAccount> {
    yggdrasil::get_account(account_key).await
}

#[command]
async fn cmd_list_yggdrasil_accounts() -> Result<HashMap<Uuid, YggdrasilAccount>> {
    yggdrasil::list_accounts().await
}

#[command]
async fn cmd_update_yggdrasil_account(account_key: Uuid, account: YggdrasilAccount) -> Result<()> {
    yggdrasil::update_account(account_key, account).await
}
