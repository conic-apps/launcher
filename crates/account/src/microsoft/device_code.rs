// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

pub async fn request_device_code() -> Result<DeviceCodeResponse> {
    Ok(HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc&scope=XboxLive.signin offline_access"
                .to_string(),
        )
        .send()
        .await?
        .json()
        .await?)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceCodePollResult {
    pub status: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

pub async fn poll_device_code(device_code: &str) -> Result<DeviceCodePollResult> {
    let response: Value = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "grant_type=urn:ietf:params:oauth:grant-type:device_code".to_string()
                + "&client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc"
                + "&device_code="
                + device_code,
        )
        .send()
        .await?
        .json()
        .await?;
    if let Some(error) = response["error"].as_str() {
        return Ok(DeviceCodePollResult {
            status: error.to_string(),
            access_token: None,
            refresh_token: None,
            expires_in: None,
        });
    }

    Ok(DeviceCodePollResult {
        status: "success".to_string(),
        access_token: response["access_token"].as_str().map(|s| s.to_string()),
        refresh_token: response["refresh_token"].as_str().map(|s| s.to_string()),
        expires_in: response["expires_in"].as_u64(),
    })
}
