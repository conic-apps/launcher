// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use folder::DATA_LOCATION;
use tauri::command;

use crate::error::*;

#[command]
pub(crate) async fn cmd_list_screenshots(instance_id: String) -> Result<Vec<String>> {
    list_scrrentshots(&instance_id)
}

pub fn list_scrrentshots(instance_id: &str) -> Result<Vec<String>> {
    let screenshots_path = DATA_LOCATION
        .get_instance_root(instance_id)
        .join("screenshots");
    Ok(std::fs::read_dir(screenshots_path)?
        .flatten()
        .flat_map(|entry| {
            if entry.path().is_file() {
                Some(entry.path().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect())
}
