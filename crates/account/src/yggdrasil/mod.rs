// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{collections::HashMap, fs::read_to_string};

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;
use url::Url;
use uuid::Uuid;

use crate::error::*;

pub mod yggdrasil_server;
pub mod yggdrasil_user_api;

pub use yggdrasil_user_api::YggdrasilAccount;

pub async fn add_account(account: YggdrasilAccount) -> Result<()> {
    let mut accounts = list_accounts().await?;
    accounts.insert(Uuid::new_v4(), account);
    save_accounts(accounts).await?;
    Ok(())
}

pub async fn delete_account(account_key: Uuid) -> Result<()> {
    let mut accounts = list_accounts().await?;
    if let Some(removed_account) = accounts.remove(&account_key) {
        let _ = yggdrasil_user_api::invalidate(
            &removed_account.api_root,
            removed_account.access_token,
            removed_account.client_token,
        )
        .await;
    };
    save_accounts(accounts).await?;
    Ok(())
}

async fn save_accounts(accounts: HashMap<Uuid, YggdrasilAccount>) -> Result<()> {
    let yggdrasil_accounts_list_file = DATA_LOCATION.accounts.join("yggdrasil-accounts.json");
    let serialized_yggdrasil_accounts_list = serde_json::to_string_pretty(&accounts)?;
    async_fs::write(
        yggdrasil_accounts_list_file,
        serialized_yggdrasil_accounts_list,
    )
    .await?;
    Ok(())
}

pub async fn list_accounts() -> Result<HashMap<Uuid, YggdrasilAccount>> {
    let yggdrasil_accounts_list_file = DATA_LOCATION.accounts.join("yggdrasil-accounts.json");
    if !yggdrasil_accounts_list_file.exists() {
        return Ok(HashMap::new());
    }
    let serialized_yggdrasil_accounts_list =
        async_fs::read_to_string(yggdrasil_accounts_list_file).await?;
    Ok(serde_json::from_str(&serialized_yggdrasil_accounts_list)?)
}

pub async fn get_account(account_key: Uuid) -> Result<YggdrasilAccount> {
    let accounts = list_accounts().await?;
    accounts
        .get(&account_key)
        .ok_or(Error::AccountNotfound(account_key))
        .cloned()
}

pub async fn update_account(account_key: Uuid, account: YggdrasilAccount) -> Result<()> {
    let mut accounts = list_accounts().await?;
    accounts.remove(&account_key);
    accounts.insert(account_key, account);
    save_accounts(accounts).await?;
    Ok(())
}
