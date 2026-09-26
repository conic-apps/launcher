// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Locally created ("offline") accounts: `crates/account/src/offline.rs` with
//! no changes beyond the ones the crate doc lists. The four commands that sat
//! in front of it (`offline_commands.rs`) only forwarded their arguments, so
//! these functions are the whole surface.

use serde::{Deserialize, Serialize};
use slint_folder::DATA_LOCATION;
use uuid::Uuid;

use crate::error::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub name: String,
    pub uuid: Uuid,
    pub skin: Option<String>,
}

impl OfflineAccount {
    fn new(name: String, uuid: Uuid) -> Self {
        Self {
            name,
            uuid,
            skin: None,
        }
    }
}

pub async fn add_account(name: String, uuid: Uuid) -> Result<()> {
    let new_account = OfflineAccount::new(name, uuid);
    let mut accounts = list_accounts()
        .await?
        .into_iter()
        .filter(|x| x.uuid != uuid)
        .collect::<Vec<_>>();
    accounts.push(new_account);
    save_accounts(&accounts).await?;
    Ok(())
}

pub async fn delete_account(uuid: Uuid) -> Result<()> {
    let accounts = list_accounts().await?;
    let result: Vec<OfflineAccount> = accounts.into_iter().filter(|x| x.uuid != uuid).collect();
    save_accounts(&result).await?;
    Ok(())
}

pub async fn update_account(account: OfflineAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let mut result = vec![];
    for old_account in accounts {
        if old_account.uuid == account.uuid {
            result.push(account.clone());
        } else {
            result.push(old_account);
        }
    }
    save_accounts(&result).await?;
    Ok(())
}

async fn save_accounts(accounts: &[OfflineAccount]) -> Result<()> {
    let accounts_list_file = DATA_LOCATION.accounts.join("offline.json");
    tokio::fs::create_dir_all(&DATA_LOCATION.accounts).await?;
    let content = serde_json::to_string(accounts)?;
    tokio::fs::write(accounts_list_file, content).await?;
    Ok(())
}

pub async fn list_accounts() -> Result<Vec<OfflineAccount>> {
    let accounts_list_file = DATA_LOCATION.accounts.join("offline.json");
    if !accounts_list_file.exists() {
        return Ok(vec![]);
    }
    let serialized_account_list = tokio::fs::read_to_string(accounts_list_file).await?;
    Ok(serde_json::from_str(&serialized_account_list)?)
}

pub async fn get_account(uuid: Uuid) -> Result<OfflineAccount> {
    let accounts = list_accounts().await?;
    accounts
        .into_iter()
        .filter(|x| x.uuid == uuid)
        .collect::<Vec<_>>()
        .first()
        .ok_or(Error::AccountNotfound(uuid))
        .cloned()
}
