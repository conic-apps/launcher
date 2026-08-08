// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::path::PathBuf;

use base64::{Engine, engine::general_purpose};
use fastnbt::Value;
use folder::DATA_LOCATION;
use tauri::command;

use crate::error::*;

pub mod datapack;
pub mod level;
mod nbt;

fn save_folder(instance_id: &str, folder_name: &str) -> PathBuf {
    DATA_LOCATION
        .get_instance_root(instance_id)
        .join("saves")
        .join(folder_name)
}

#[command]
pub(crate) async fn cmd_get_all_levels(instance_id: &str) -> Result<HashMap<String, Value>> {
    level::get_all_levels(DATA_LOCATION.get_instance_root(instance_id).join("saves"))
}

#[command]
pub(crate) async fn cmd_get_save_icon(instance_id: &str, folder_name: &str) -> Result<String> {
    let icon_path = save_folder(instance_id, folder_name).join("icon.png");
    let icon = async_fs::read(icon_path).await?;
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(icon)
    ))
}

#[command]
pub(crate) async fn cmd_get_save_path(instance_id: &str, folder_name: &str) -> Result<String> {
    Ok(save_folder(instance_id, folder_name)
        .to_string_lossy()
        .to_string())
}

#[command]
pub(crate) async fn cmd_delete_save(instance_id: &str, folder_name: &str) -> Result<()> {
    async_fs::remove_dir_all(save_folder(instance_id, folder_name)).await?;
    Ok(())
}
