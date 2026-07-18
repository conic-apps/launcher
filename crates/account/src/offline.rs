// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub name: String,
    pub uuid: Uuid,
    pub skin: Option<String>,
}

impl OfflineAccount {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: uuid::Uuid::from_u128(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Incorrect System Time")
                    .as_nanos(),
            ),
            skin: None,
        }
    }
}

pub async fn add_account(name: &str) -> Result<()> {
    let new_account = OfflineAccount::new(name);
    let mut accounts = list_accounts().await?;
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

async fn save_accounts(accounts: &Vec<OfflineAccount>) -> Result<()> {
    let accounts_list_file = DATA_LOCATION.accounts.join("offline.json");
    async_fs::create_dir_all(&DATA_LOCATION.accounts).await?;
    let content = serde_json::to_string(accounts)?;
    async_fs::write(accounts_list_file, content).await?;
    Ok(())
}

pub async fn list_accounts() -> Result<Vec<OfflineAccount>> {
    let accounts_list_file = DATA_LOCATION.accounts.join("offline.json");
    if !accounts_list_file.exists() {
        return Ok(vec![]);
    }
    let serialized_account_list = async_fs::read_to_string(accounts_list_file).await?;
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
