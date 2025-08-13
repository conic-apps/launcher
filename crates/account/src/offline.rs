// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    fs::create_dir_all,
    time::{SystemTime, UNIX_EPOCH},
};

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

pub fn add_account(name: &str) -> Result<()> {
    let new_account = OfflineAccount::new(name);
    let mut accounts = list_accounts()?;
    accounts.push(new_account);
    save_accounts(&accounts)?;
    Ok(())
}

pub fn delete_account(uuid: Uuid) -> Result<()> {
    let accounts = list_accounts()?;
    let result: Vec<OfflineAccount> = accounts.into_iter().filter(|x| x.uuid != uuid).collect();
    save_accounts(&result)?;
    Ok(())
}

pub fn update_account(account: OfflineAccount) -> Result<()> {
    let accounts = list_accounts()?;
    let mut result = vec![];
    for old_account in accounts {
        if old_account.uuid == account.uuid {
            result.push(account.clone());
        } else {
            result.push(old_account);
        }
    }
    save_accounts(&result)?;
    Ok(())
}

fn save_accounts(accounts: &Vec<OfflineAccount>) -> Result<()> {
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let content = serde_json::to_string(accounts)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn list_accounts() -> Result<Vec<OfflineAccount>> {
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn get_account(uuid: Uuid) -> Result<OfflineAccount> {
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    if !path.exists() {
        return Err(Error::AccountNotfound(uuid));
    };
    let data = std::fs::read_to_string(path)?;
    let accounts = serde_json::from_str::<Vec<OfflineAccount>>(&data)?;
    accounts
        .into_iter()
        .filter(|x| x.uuid == uuid)
        .collect::<Vec<_>>()
        .first()
        .ok_or(Error::AccountNotfound(uuid))
        .cloned()
}
