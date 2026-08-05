// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use base64::{Engine, engine::general_purpose};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;
use uuid::Uuid;

use crate::error::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub state: String,
    #[serde(rename(serialize = "textureKey", deserialize = "textureKey"))]
    pub texture_key: String,
    pub url: String,
    pub variant: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Cape {
    pub alias: String,
    pub id: String,
    pub state: String,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub profile_name: String,
    pub uuid: Uuid,
    pub skins: Vec<Skin>,
    pub capes: Vec<Cape>,
}

pub async fn generate_account_profile(minecraft_profile_response: Value) -> Result<Profile> {
    Ok(Profile {
        profile_name: serde_json::from_value(minecraft_profile_response["name"].clone())?,
        uuid: serde_json::from_value(minecraft_profile_response["id"].clone())?,
        skins: resolve_skins(serde_json::from_value(
            minecraft_profile_response["skins"].clone(),
        )?)
        .await,
        capes: resolve_capes(serde_json::from_value(
            minecraft_profile_response["capes"].clone(),
        )?)
        .await,
    })
}

async fn resolve_skins(skins: Vec<Skin>) -> Vec<Skin> {
    let mut result = Vec::with_capacity(skins.len());
    for skin in skins {
        let mut skin = skin.clone();
        skin.url = resolve_skin(&skin.url).await;
        result.push(skin);
    }
    result
}

async fn resolve_skin(url: &str) -> String {
    async fn download_skin(url: &str) -> Result<Vec<u8>> {
        Ok(HTTP_CLIENT.get(url).send().await?.bytes().await?.to_vec())
    }
    if let Ok(content) = download_skin(url).await {
        format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD_NO_PAD.encode(content)
        )
    } else {
        url.to_string()
    }
}

async fn resolve_capes(capes: Vec<Cape>) -> Vec<Cape> {
    let futures: Vec<_> = capes.into_iter().map(resolve_cape).collect();
    stream::iter(futures)
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await
}

async fn resolve_cape(cape: Cape) -> Cape {
    async fn download_cape(url: &str) -> Result<Vec<u8>> {
        Ok(HTTP_CLIENT.get(url).send().await?.bytes().await?.to_vec())
    }
    let url = if let Ok(content) = download_cape(&cape.url).await {
        format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD_NO_PAD.encode(content)
        )
    } else {
        cape.url
    };
    Cape {
        alias: cape.alias,
        id: cape.id,
        state: cape.state,
        url,
    }
}
