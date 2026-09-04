// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use folder::DATA_LOCATION;
use log::warn;
use serde::{Deserialize, Serialize};
use tauri::command;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Favorite {
    pub platform: String,
    pub content_type: String,
    pub project_id: String,
}

fn favorites_path() -> std::path::PathBuf {
    DATA_LOCATION.root.join("favorites.json")
}

fn read_favorites() -> Result<Vec<Favorite>> {
    let path = favorites_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&path)?;
    let favorites: Vec<Favorite> = serde_json::from_str(&data)?;
    Ok(favorites)
}

fn write_favorites(favorites: &[Favorite]) -> Result<()> {
    let path = favorites_path();
    let data = serde_json::to_string_pretty(favorites)?;
    std::fs::write(&path, data)?;
    Ok(())
}

#[command]
pub fn cmd_list_favorites() -> Result<Vec<Favorite>> {
    read_favorites()
}

#[command]
pub fn cmd_add_favorite(platform: String, content_type: String, project_id: String) -> Result<()> {
    let mut favorites = read_favorites()?;
    let new_favorite = Favorite {
        platform,
        content_type,
        project_id,
    };
    if !favorites.contains(&new_favorite) {
        favorites.push(new_favorite);
        if let Err(error) = write_favorites(&favorites) {
            warn!("Failed to write favorites: {error}");
            return Err(error);
        }
    }
    Ok(())
}

#[command]
pub fn cmd_remove_favorite(
    platform: String,
    content_type: String,
    project_id: String,
) -> Result<()> {
    let mut favorites = read_favorites()?;
    let target = Favorite {
        platform,
        content_type,
        project_id,
    };
    favorites.retain(|f| f != &target);
    if let Err(error) = write_favorites(&favorites) {
        warn!("Failed to write favorites: {error}");
        return Err(error);
    }
    Ok(())
}

#[command]
pub fn cmd_is_favorited(
    platform: String,
    content_type: String,
    project_id: String,
) -> Result<bool> {
    let favorites = read_favorites()?;
    let target = Favorite {
        platform,
        content_type,
        project_id,
    };
    Ok(favorites.contains(&target))
}
