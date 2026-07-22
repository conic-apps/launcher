// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use tauri::command;
use uuid::Uuid;

use crate::{Result, offline::OfflineAccount};

#[command]
pub(crate) async fn cmd_offline_add_account(name: String, uuid: Uuid) -> Result<()> {
    crate::offline::add_account(name, uuid).await
}

#[command]
pub(crate) async fn cmd_offline_delete_account(uuid: Uuid) -> Result<()> {
    crate::offline::delete_account(uuid).await
}

#[command]
pub(crate) async fn cmd_offline_update_account(account: OfflineAccount) -> Result<()> {
    crate::offline::update_account(account).await
}

#[command]
pub(crate) async fn cmd_offline_get_account(uuid: Uuid) -> Result<OfflineAccount> {
    crate::offline::get_account(uuid).await
}
