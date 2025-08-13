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
    authlib_injector::{AuthlibInjectorAccount, LoginResponse, Profile, YggdrasilServerInfo},
    microsoft::MicrosoftAccount,
    offline::OfflineAccount,
};
use error::*;

pub mod authlib_injector;
pub mod error;
pub mod microsoft;
pub mod offline;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountType {
    Microsoft,
    Offline,
    AuthlibInjector,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("account")
        .invoke_handler(tauri::generate_handler![
            cmd_list_accounts,
            cmd_add_microsoft_account,
            cmd_delete_microsoft_account,
            cmd_refresh_all_microsoft_accounts,
            cmd_refresh_microsoft_account,
            cmd_get_microsoft_account,
            cmd_add_offline_account,
            cmd_delete_offline_account,
            cmd_update_offline_account,
            cmd_get_offline_account,
            cmd_add_yggdrasil_server,
            cmd_delete_yggdrasil_server,
            cmd_list_yggdrasil_server,
            cmd_get_yggdrasil_server_info,
            cmd_yggdrasil_login,
            cmd_add_authlib_account,
            cmd_delete_authlib_account,
            cmd_get_authlib_profile_info,
            cmd_get_authlib_account,
            cmd_relogin_account,
        ])
        .build()
}

#[derive(Serialize, Deserialize)]
struct Accounts {
    microsoft: Vec<MicrosoftAccount>,
    offline: Vec<OfflineAccount>,
    authlib_injector: HashMap<Uuid, AuthlibInjectorAccount>,
}

#[command]
fn cmd_list_accounts() -> Result<Accounts> {
    Ok(Accounts {
        microsoft: microsoft::list_accounts()?,
        offline: offline::list_accounts()?,
        authlib_injector: authlib_injector::list_accounts()?,
    })
}

pub struct AccountLaunchInfo {
    pub access_token: String,
    pub name: String,
    pub uuid: String,
}

impl AccountLaunchInfo {
    pub fn new(uuid: Uuid, account_type: &AccountType) -> Result<Self> {
        let result = match account_type {
            AccountType::Microsoft => {
                let microsoft_account = microsoft::get_account(uuid)?;
                Self {
                    access_token: microsoft_account.access_token,
                    name: microsoft_account.profile.profile_name,
                    uuid: microsoft_account.profile.uuid.to_string(),
                }
            }
            AccountType::AuthlibInjector => {
                let authlib_account = authlib_injector::get_account(uuid)?;
                Self {
                    access_token: authlib_account.access_token,
                    name: authlib_account.profile_name,
                    uuid: authlib_account.profile_uuid.to_string(),
                }
            }
            AccountType::Offline => {
                let offline_account = offline::get_account(uuid)?;
                Self {
                    access_token: "1145141919810".to_string(),
                    name: offline_account.name,
                    uuid: offline_account.uuid.to_string(),
                }
            }
        };
        Ok(result)
    }
}

// TODO: Errors: relogin microsoft, relogin authlib
pub async fn force_refresh_account(uuid: Uuid, account_type: &AccountType) -> Result<()> {
    match account_type {
        AccountType::Microsoft => {
            microsoft::refresh_account(uuid).await?;
        }
        AccountType::AuthlibInjector => authlib_injector::refresh_account(uuid).await?,
        AccountType::Offline => (),
    };
    Ok(())
}

// TODO: Errors: relogin microsoft, relogin authlib
pub async fn check_and_refresh_account(uuid: Uuid, account_type: &AccountType) -> Result<()> {
    match account_type {
        AccountType::Microsoft => {
            microsoft::check_and_refresh_account(uuid).await?;
        }
        AccountType::AuthlibInjector => {
            authlib_injector::check_and_refresh_account(uuid).await?;
        }
        AccountType::Offline => (),
    };
    Ok(())
}

#[command]
async fn cmd_relogin_account(
    uuid: Uuid,
    account_type: AccountType,
    credential: String,
) -> Result<()> {
    match account_type {
        AccountType::Microsoft => microsoft::relogin_account(uuid, credential).await?,
        AccountType::AuthlibInjector => authlib_injector::relogin_account(uuid, credential).await?,
        AccountType::Offline => (),
    };
    Ok(())
}

#[command]
fn cmd_get_microsoft_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    microsoft::get_account(uuid)
}

#[command]
fn cmd_delete_microsoft_account(uuid: Uuid) -> Result<()> {
    microsoft::delete_account(uuid)
}

#[command]
async fn cmd_refresh_all_microsoft_accounts() -> Result<()> {
    microsoft::refresh_all_accounts().await
}

#[command]
async fn cmd_refresh_microsoft_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    microsoft::refresh_account(uuid).await
}

#[command]
async fn cmd_add_microsoft_account(code: String) -> Result<()> {
    microsoft::add_account(code).await
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
    authlib_injector::add_yggdrasil_server(&api_root).await
}

#[command]
fn cmd_delete_yggdrasil_server(index_to_delete: usize) -> Result<()> {
    authlib_injector::delete_yggdrasil_server(index_to_delete)
}

#[command]
fn cmd_list_yggdrasil_server() -> Result<Vec<String>> {
    authlib_injector::list_yggdrasil_server()
}

#[command]
async fn cmd_get_yggdrasil_server_info(api_root: String) -> Result<YggdrasilServerInfo> {
    authlib_injector::get_yggdrasil_server_info(&api_root).await
}

#[command]
async fn cmd_yggdrasil_login(
    api_root: String,
    username: String,
    password: String,
) -> Result<LoginResponse> {
    authlib_injector::login(&api_root, username, password).await
}

#[command]
fn cmd_add_authlib_account(account: AuthlibInjectorAccount) -> Result<()> {
    authlib_injector::add_account(account)
}

#[command]
async fn cmd_delete_authlib_account(account_key: Uuid) -> Result<()> {
    authlib_injector::delete_account(account_key).await
}

#[command]
async fn cmd_get_authlib_profile_info(api_root: String, uuid: Uuid) -> Result<Profile> {
    authlib_injector::get_profile_info(&api_root, uuid).await
}
#[command]
fn cmd_get_authlib_account(account_key: Uuid) -> Result<AuthlibInjectorAccount> {
    authlib_injector::get_account(account_key)
}
