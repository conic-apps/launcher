// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

async fn check_response(response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    serde_json::from_str(&body).map_err(Into::into)
}

pub async fn redeem_access_token(code: &str) -> Result<TokenPair> {
    let response = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=authorization_code"
                + "&code="
                + code
                + "&redirect_uri=conic-launcher%3A%2F%2Foauth2%2Fmicrosoft%2Fcallback"
                + "&scope=XboxLive.signin%20offline_access",
        )
        .send()
        .await?;
    let response = check_response(response).await?;
    let access_token = response["access_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "access_token".to_string(),
        ))?
        .to_string();
    let refresh_token = response["refresh_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "refresh_token".to_string(),
        ))?
        .to_string();
    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}

pub(super) async fn get_access_token_from_refresh_token(refresh_token: &str) -> Result<TokenPair> {
    let response = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=refresh_token"
                + "&refresh_token="
                + refresh_token
                + "&redirect_uri=conic-launcher%3A%2F%2Foauth2%2Fmicrosoft%2Fcallback"
                + "&scope=XboxLive.signin%20offline_access",
        )
        .send()
        .await?;
    let response = check_response(response).await?;
    let access_token = response["access_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "access_token".to_string(),
        ))?
        .to_string();
    let refresh_token = response["refresh_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "refresh_token".to_string(),
        ))?
        .to_string();
    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}
