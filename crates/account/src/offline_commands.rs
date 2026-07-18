// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use tauri::command;
use uuid::Uuid;

use crate::{Result, offline::OfflineAccount};

#[command]
pub async fn cmd_add_offline_account(name: String) -> Result<()> {
    crate::offline::add_account(&name).await
}

#[command]
pub async fn cmd_delete_offline_account(uuid: Uuid) -> Result<()> {
    crate::offline::delete_account(uuid).await
}

#[command]
pub async fn cmd_update_offline_account(account: OfflineAccount) -> Result<()> {
    crate::offline::update_account(account).await
}

#[command]
pub async fn cmd_get_offline_account(uuid: Uuid) -> Result<OfflineAccount> {
    crate::offline::get_account(uuid).await
}
