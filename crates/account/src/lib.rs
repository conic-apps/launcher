// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};

use crate::microsoft::MicrosoftAccount;

pub mod microsoft;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountType {
    Microsoft,
    Offline,
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
        ])
        .build()
}

#[derive(Serialize, Deserialize)]
struct Accounts {
    microsoft: Vec<MicrosoftAccount>,
    // TODO: Offline accounts
}

#[command]
fn cmd_list_accounts() -> Accounts {
    Accounts {
        microsoft: microsoft::list_accounts(),
    }
}

pub struct AccountLaunchInfo {
    pub access_token: String,
    pub name: String,
    pub uuid: String,
}

impl AccountLaunchInfo {
    pub fn new(uuid: &str, account_type: &AccountType) -> Self {
        match account_type {
            AccountType::Microsoft => {
                let microsoft_account = microsoft::get_account(uuid).first().unwrap().clone();
                Self {
                    access_token: microsoft_account.access_token,
                    name: microsoft_account.profile.profile_name,
                    uuid: microsoft_account.profile.uuid,
                }
            }
            AccountType::Offline => {
                todo!()
            }
        }
    }
}

pub async fn force_refresh_account(uuid: &str, account_type: &AccountType) {
    match account_type {
        AccountType::Microsoft => microsoft::refresh_account(uuid.to_string()).await,
        AccountType::Offline => todo!(),
    };
}

pub async fn check_and_refresh_account(
    uuid: &str,
    account_type: &AccountType,
) -> anyhow::Result<()> {
    match account_type {
        AccountType::Microsoft => microsoft::check_and_refresh_account(uuid).await?,
        AccountType::Offline => todo!(),
    };
    Ok(())
}

#[command]
fn cmd_get_microsoft_account(uuid: String) -> Vec<MicrosoftAccount> {
    microsoft::get_account(&uuid)
}

#[command]
fn cmd_delete_microsoft_account(uuid: String) {
    microsoft::delete_account(uuid)
}

#[command]
async fn cmd_refresh_all_microsoft_accounts() {
    microsoft::refresh_all_accounts().await
}

#[command]
async fn cmd_refresh_microsoft_account(uuid: String) -> MicrosoftAccount {
    microsoft::refresh_account(uuid).await
}

#[command]
async fn cmd_add_microsoft_account(code: String) {
    microsoft::add_account(code).await.unwrap();
}

// TODO: add yggdrasil account
// TODO: add offline account
