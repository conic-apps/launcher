// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde_json::Value;

use crate::error::*;
use crate::shared::HTTP_CLIENT;

pub(super) async fn get_game_profile(minecraft_access_token: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {minecraft_access_token}"))
        .send()
        .await?
        .json()
        .await?)
}
