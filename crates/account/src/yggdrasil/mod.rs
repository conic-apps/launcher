// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use folder::DATA_LOCATION;
use uuid::Uuid;

use crate::error::*;

pub mod yggdrasil_server;
pub mod yggdrasil_user_api;

pub use yggdrasil_user_api::YggdrasilAccount;

pub async fn add_account(account: YggdrasilAccount) -> Result<()> {
    let mut accounts = list_accounts()
        .await?
        .into_iter()
        .filter(|x| {
            !(x.api_root == account.api_root
                && x.profile.name == account.profile.name
                && x.profile.id == account.profile.id)
        })
        .collect::<Vec<_>>();
    accounts.push(account);
    save_accounts(accounts).await?;
    Ok(())
}

pub async fn delete_account(account: YggdrasilAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .filter(|x| x.identifier != account.identifier)
        .collect::<Vec<_>>();
    let _ = yggdrasil_user_api::invalidate(
        &account.api_root,
        account.access_token,
        account.client_token,
    )
    .await;
    save_accounts(result).await?;
    Ok(())
}

async fn save_accounts(accounts: Vec<YggdrasilAccount>) -> Result<()> {
    let yggdrasil_accounts_list_file = DATA_LOCATION.accounts.join("yggdrasil-accounts.json");
    async_fs::create_dir_all(&DATA_LOCATION.accounts).await?;
    let serialized_yggdrasil_accounts_list = serde_json::to_string_pretty(&accounts)?;
    async_fs::write(
        yggdrasil_accounts_list_file,
        serialized_yggdrasil_accounts_list,
    )
    .await?;
    Ok(())
}

pub async fn list_accounts() -> Result<Vec<YggdrasilAccount>> {
    let yggdrasil_accounts_list_file = DATA_LOCATION.accounts.join("yggdrasil-accounts.json");
    async_fs::create_dir_all(&DATA_LOCATION.accounts).await?;
    if !yggdrasil_accounts_list_file.exists() {
        return Ok(vec![]);
    }
    let serialized_yggdrasil_accounts_list = async_fs::read_to_string(yggdrasil_accounts_list_file)
        .await
        .unwrap_or_default();
    Ok(serde_json::from_str(&serialized_yggdrasil_accounts_list).unwrap_or_default())
}

pub async fn get_account(account_identifier: Uuid) -> Result<YggdrasilAccount> {
    let accounts = list_accounts().await?;
    accounts
        .into_iter()
        .find(|account| account.identifier == account_identifier)
        .ok_or(Error::AccountNotfound(account_identifier))
}

pub async fn update_account(account_identifier: Uuid, account: YggdrasilAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .map(|x| {
            if x.identifier == account_identifier {
                account.clone()
            } else {
                x
            }
        })
        .collect::<Vec<_>>();
    save_accounts(result).await?;
    Ok(())
}
