use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

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
