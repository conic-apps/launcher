// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;
use serde_json::Value;

use crate::error::*;
use shared::HTTP_CLIENT;

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

/// The body the cape endpoint takes: `{"capeId": "…"}`.
#[derive(Serialize)]
struct ActiveCapeBody<'a> {
    #[serde(rename = "capeId")]
    cape_id: &'a str,
}

/// Makes one of the profile's capes the active one.
///
/// Unlike the older skin upload (which took the cape alongside the texture),
/// Mojang exposes capes as their own sub-resource, and this is the same
/// `PUT /minecraft/profile/capes/active` the MultiMC/Prism launchers drive. The
/// cape has to be one the account already owns, or the server answers 400.
pub(super) async fn set_active_cape(minecraft_access_token: &str, cape_id: &str) -> Result<()> {
    let response = HTTP_CLIENT
        .put("https://api.minecraftservices.com/minecraft/profile/capes/active")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {minecraft_access_token}"))
        .json(&ActiveCapeBody { cape_id })
        .send()
        .await?;
    ensure_success(response).await
}

/// Clears the active cape. The account keeps owning every cape; this only turns
/// the enabled one off.
pub(super) async fn clear_active_cape(minecraft_access_token: &str) -> Result<()> {
    let response = HTTP_CLIENT
        .delete("https://api.minecraftservices.com/minecraft/profile/capes/active")
        .header("Authorization", format!("Bearer {minecraft_access_token}"))
        .send()
        .await?;
    ensure_success(response).await
}

/// Reads the response once: a success is silent, a failure carries the
/// server's own body (its `errorMessage`, or the bare error) so the caller can
/// show it.
async fn ensure_success(response: reqwest::Response) -> Result<()> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    Ok(())
}
