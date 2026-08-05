// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

#[derive(Clone, Serialize, Deserialize)]
struct MinecraftAuthBody {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

impl MinecraftAuthBody {
    fn new(xbl_uhs: &str, xsts_token: &str) -> Self {
        Self {
            identity_token: format!("XBL3.0 x={xbl_uhs}; {xsts_token}"),
        }
    }
}

pub(super) async fn minecraft_authenticate(
    xbl_uhs: &str,
    xsts_token: &str,
) -> Result<(String, u64)> {
    let response: Value = HTTP_CLIENT
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(serde_json::to_string(&MinecraftAuthBody::new(
            xbl_uhs, xsts_token,
        ))?)
        .send()
        .await?
        .json()
        .await?;
    Ok((
        response["access_token"]
            .as_str()
            .ok_or(Error::MicrosoftResponseMissingKey(
                "access_token".to_string(),
            ))?
            .to_string(),
        response["expires_in"]
            .as_u64()
            .ok_or(Error::MicrosoftResponseMissingKey("expires_in".to_string()))?,
    ))
}
