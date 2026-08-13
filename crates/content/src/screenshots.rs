// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use folder::DATA_LOCATION;
use tauri::command;

use crate::error::*;

const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

#[command]
pub(crate) async fn cmd_list_screenshots(instance_id: String) -> Result<Vec<String>> {
    list_screenshots(&instance_id)
}

pub fn list_screenshots(instance_id: &str) -> Result<Vec<String>> {
    let screenshots_path = DATA_LOCATION
        .get_instance_root(instance_id)
        .join("screenshots");
    let mut screenshots = std::fs::read_dir(screenshots_path)?
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter(|entry| is_image(&entry.path()))
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    screenshots.sort();
    Ok(screenshots)
}

fn is_image(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
}
