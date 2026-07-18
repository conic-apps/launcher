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

use crate::{microsoft::MicrosoftAccount, offline::OfflineAccount, yggdrasil::YggdrasilAccount};
pub use error::*;

mod error;
pub mod microsoft;
mod microsoft_commands;
pub mod offline;
mod offline_commands;
pub mod yggdrasil;
mod yggdrasil_commands;

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
            Account::Offline(_) => "114514".to_string(),
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
        .invoke_handler(tauri::generate_handler![
            cmd_list_accounts,
            microsoft_commands::cmd_get_microsoft_account,
            microsoft_commands::cmd_delete_microsoft_account,
            microsoft_commands::cmd_add_microsoft_account,
            microsoft_commands::cmd_update_microsoft_account,
            microsoft_commands::cmd_redeem_access_token,
            microsoft_commands::cmd_microsoft_access_token_auth_flow,
            microsoft_commands::cmd_refresh_microsoft_account,
            microsoft_commands::cmd_request_device_code,
            microsoft_commands::cmd_poll_device_code,
            offline_commands::cmd_add_offline_account,
            offline_commands::cmd_delete_offline_account,
            offline_commands::cmd_update_offline_account,
            offline_commands::cmd_get_offline_account,
            yggdrasil_commands::cmd_add_yggdrasil_server,
            yggdrasil_commands::cmd_delete_yggdrasil_server,
            yggdrasil_commands::cmd_list_yggdrasil_server,
            yggdrasil_commands::cmd_get_yggdrasil_server_info,
            yggdrasil_commands::cmd_yggdrasil_authenticate_account,
            yggdrasil_commands::cmd_yggdrasil_validate_account,
            yggdrasil_commands::cmd_yggdrasil_refresh_account,
            yggdrasil_commands::cmd_yggdrasil_invalidate_account,
            yggdrasil_commands::cmd_yggdrasil_get_profile,
            yggdrasil_commands::cmd_add_yggdrasil_account,
            yggdrasil_commands::cmd_delete_yggdrasil_account,
            yggdrasil_commands::cmd_get_yggdrasil_account,
            yggdrasil_commands::cmd_list_yggdrasil_accounts,
            yggdrasil_commands::cmd_update_yggdrasil_account,
        ])
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
        offline: offline::list_accounts().await?,
        third_party_yggdrasil: yggdrasil::list_accounts().await?,
    })
}
