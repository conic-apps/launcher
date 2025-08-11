// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub name: String,
    pub uuid: String,
    pub skin: Option<String>,
}

impl OfflineAccount {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: uuid::Uuid::from_u128(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            )
            .to_string(),
            skin: None,
        }
    }
}

pub fn add_account(name: &str) {
    let new_account = OfflineAccount::new(name);
    let mut accounts = list_accounts();
    accounts.push(new_account);
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    let content = serde_json::to_string(&accounts).unwrap();
    std::fs::write(path, content).unwrap();
}

pub fn delete_account(uuid: &str) {
    let accounts = list_accounts();
    let result: Vec<OfflineAccount> = accounts.into_iter().filter(|x| x.uuid != uuid).collect();
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    let contents = serde_json::to_string(&result).unwrap();
    std::fs::write(path, contents).unwrap();
}

pub fn update_account(account: OfflineAccount) {
    let accounts = list_accounts();
    let mut result = vec![];
    for old_account in accounts {
        if old_account.uuid == account.uuid {
            result.push(account.clone());
        } else {
            result.push(old_account);
        }
    }
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    let contents = serde_json::to_string(&result).unwrap();
    std::fs::write(path, contents).unwrap();
}

pub fn list_accounts() -> Vec<OfflineAccount> {
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    let data = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&data).unwrap()
}

pub fn get_account(uuid: &str) -> Vec<OfflineAccount> {
    let path = DATA_LOCATION.root.join("accounts.offline.json");
    if !path.exists() {
        return vec![];
    };
    let data = std::fs::read_to_string(path).unwrap();
    let accounts = serde_json::from_str::<Vec<OfflineAccount>>(&data).unwrap();
    accounts.into_iter().filter(|x| x.uuid == uuid).collect()
}
