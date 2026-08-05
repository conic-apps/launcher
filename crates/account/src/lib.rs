// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use base64::{Engine, engine::general_purpose};
use serde::{Deserialize, Serialize};
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};

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
#[serde(tag = "type", content = "data")]
pub enum Account {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
    Yggdrasil(YggdrasilAccount),
}

impl Account {
    pub fn get_profile_name(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.profile_name.to_string(),
            Account::Yggdrasil(account) => account.profile.name.to_string(),
            Account::Offline(account) => account.name.to_string(),
        }
    }

    pub fn get_profile_uuid(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.uuid.to_string(),
            Account::Yggdrasil(account) => account.profile.id.to_string(),
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
            cmd_save_skin,
            microsoft_commands::cmd_microsoft_get_account,
            microsoft_commands::cmd_microsoft_delete_account,
            microsoft_commands::cmd_microsoft_add_account,
            microsoft_commands::cmd_microsoft_update_account,
            microsoft_commands::cmd_microsoft_redeem_access_token,
            microsoft_commands::cmd_microsoft_access_token_auth_flow,
            microsoft_commands::cmd_microsoft_refresh_account,
            microsoft_commands::cmd_microsoft_request_device_code,
            microsoft_commands::cmd_microsoft_poll_device_code,
            offline_commands::cmd_offline_add_account,
            offline_commands::cmd_offline_delete_account,
            offline_commands::cmd_offline_update_account,
            offline_commands::cmd_offline_get_account,
            yggdrasil_commands::cmd_yggdrasil_get_server_info,
            yggdrasil_commands::cmd_yggdrasil_authenticate_account,
            yggdrasil_commands::cmd_yggdrasil_validate_account,
            yggdrasil_commands::cmd_yggdrasil_refresh_account,
            yggdrasil_commands::cmd_yggdrasil_invalidate_account,
            yggdrasil_commands::cmd_yggdrasil_get_profile,
            yggdrasil_commands::cmd_yggdrasil_get_profiles,
            yggdrasil_commands::cmd_yggdrasil_add_account,
            yggdrasil_commands::cmd_yggdrasil_delete_account,
            yggdrasil_commands::cmd_yggdrasil_get_account,
            yggdrasil_commands::cmd_yggdrasil_list_accounts,
            yggdrasil_commands::cmd_yggdrasil_update_account,
        ])
        .build()
}

#[derive(Serialize, Deserialize)]
struct Accounts {
    microsoft: Vec<MicrosoftAccount>,
    offline: Vec<OfflineAccount>,
    yggdrasil: Vec<YggdrasilAccount>,
}

#[command]
async fn cmd_list_accounts() -> Accounts {
    Accounts {
        microsoft: microsoft::list_accounts().await.unwrap_or_default(),
        offline: offline::list_accounts().await.unwrap_or_default(),
        yggdrasil: yggdrasil::list_accounts().await.unwrap_or_default(),
    }
}

#[command]
async fn cmd_save_skin(base64_skin_url: String, path: String) -> Result<()> {
    let data = base64_skin_url
        .split_once(',')
        .map(|(_, data)| data)
        .unwrap_or(&base64_skin_url)
        .to_string();
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(&data)
        .or_else(|_| general_purpose::STANDARD.decode(data))?;
    async_fs::write(path, bytes).await?;
    Ok(())
}
