// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use fastnbt::Value;
use folder::DATA_LOCATION;
use tauri::command;
use uuid::Uuid;

use crate::error::*;

pub mod datapack;
pub mod level;
mod nbt;

#[command]
pub(crate) async fn cmd_get_all_levels(instance_id: Uuid) -> Result<HashMap<String, Value>> {
    level::get_all_levels(DATA_LOCATION.get_instance_root(&instance_id).join("saves"))
}
