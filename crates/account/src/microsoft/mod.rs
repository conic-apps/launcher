// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{error::*, microsoft::account_profile_step::Profile};
use folder::DATA_LOCATION;
use shared::HTTP_CLIENT;

mod account_profile_step;
pub(crate) mod device_code;
mod microsoft_auth_step;
mod minecraft_auth_step;
mod minecraft_profile_step;
mod xbox_auth_step;
mod xsts_auth_step;

pub use microsoft_auth_step::get_access_token;

#[derive(Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub refresh_token: String,
    pub minecraft_access_token: String,
    pub expires_at: u64,
    pub profile: Profile,
}

pub async fn list_accounts() -> Result<Vec<MicrosoftAccount>> {
    let accounts_list_file = DATA_LOCATION.accounts.join("microsoft.json");
    if !accounts_list_file.exists() {
        return Ok(vec![]);
    }
    let serialized_account_list = async_fs::read_to_string(accounts_list_file).await?;
    Ok(serde_json::from_str(&serialized_account_list)?)
}

pub async fn get_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    let accounts_list_file = DATA_LOCATION.accounts.join("microsoft.json");
    if !accounts_list_file.exists() {
        return Err(Error::AccountNotfound(uuid));
    }
    let serialized_accounts_list = async_fs::read_to_string(accounts_list_file).await?;
    let accounts = serde_json::from_str::<Vec<MicrosoftAccount>>(&serialized_accounts_list)?;
    accounts
        .into_iter()
        .filter(|x| x.profile.uuid == uuid)
        .collect::<Vec<_>>()
        .first()
        .ok_or(Error::AccountNotfound(uuid))
        .cloned()
}

pub async fn add_account(account: MicrosoftAccount) -> Result<()> {
    let mut accounts = list_accounts().await?;
    accounts.push(account);
    let accounts_list_file = DATA_LOCATION.accounts.join("microsoft.json");
    let serialized_accounts_list = serde_json::to_string_pretty(&accounts)?;
    async_fs::write(accounts_list_file, serialized_accounts_list).await?;
    Ok(())
}

pub async fn delete_account(uuid: Uuid) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .filter(|x| x.profile.uuid != uuid)
        .collect::<Vec<MicrosoftAccount>>();
    let accounts_list_file = DATA_LOCATION.accounts.join("microsoft.json");
    let serialized_accounts_list = serde_json::to_string_pretty(&result)?;
    async_fs::write(accounts_list_file, serialized_accounts_list).await?;
    Ok(())
}

pub async fn update_account(uuid: Uuid, account: &MicrosoftAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .map(|item| {
            if account.profile.uuid == uuid {
                return account.clone();
            } else {
                return item;
            }
        })
        .collect::<Vec<MicrosoftAccount>>();
    let accounts_list_file = DATA_LOCATION.accounts.join("microsoft.json");
    let serialized_accounts_list = serde_json::to_string_pretty(&result)?;
    async_fs::write(accounts_list_file, serialized_accounts_list).await?;
    Ok(())
}

pub async fn refresh_account(uuid: Uuid, force_refresh: bool) -> Result<MicrosoftAccount> {
    let account = get_account(uuid).await?;
    if !force_refresh {
        info!("Checking account: {uuid}");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Incorrect System Time")
            .as_secs();
        const AHEAD: u64 = 3600 * 4;
        if now <= account.expires_at - AHEAD {
            info!(
                "The access token will expire in {} seconds, no need to refresh.",
                account.expires_at - now
            );
            return Ok(account.clone());
        }
    }
    info!("Start refreshing the account: {uuid}");
    let (access_token, refresh_token) =
        microsoft_auth_step::get_access_token_from_refresh_token(&account.refresh_token).await?;
    let refreshed_account = access_token_auth_flow(&access_token, &refresh_token).await?;
    update_account(uuid, &refreshed_account);
    Ok(refreshed_account)
}

pub async fn access_token_auth_flow(
    access_token: &str,
    refresh_token: &str,
) -> Result<MicrosoftAccount> {
    let xbox_auth_response = xbox_auth_step::xbox_authenticate(&access_token).await?;
    info!("Successfully login Xbox");

    let xsts_token = xsts_auth_step::xsts_authenticate(&xbox_auth_response.xbl_token).await?;
    info!("Successfully verify XSTS");

    let (minecraft_access_token, expires_in_secs) =
        minecraft_auth_step::minecraft_authenticate(&xbox_auth_response.xbl_uhs, &xsts_token)
            .await?;
    info!("Successfully get Minecraft access token");

    let minecraft_profile_response =
        minecraft_profile_step::get_game_profile(&minecraft_access_token).await?;
    info!("Successfully get game profile");

    Ok(MicrosoftAccount {
        refresh_token: refresh_token.to_string(),
        minecraft_access_token,
        expires_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Incorrect System Time")
            .as_secs()
            + expires_in_secs,
        profile: account_profile_step::generate_account_profile(minecraft_profile_response).await?,
    })
}
