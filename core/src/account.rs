// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_plugin_http::reqwest;

use crate::config::account::Profile;

async fn get_access_token(client: &reqwest::Client, code: &str) -> anyhow::Result<Value> {
    Ok(client
        .post("https://login.live.com/oauth20_token.srf")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=00000000402b5328".to_string()
                + "&grant_type=authorization_code"
                + "&code="
                + code
                + "&redirect_uri=https://login.live.com/oauth20_desktop.srf"
                + "&scope=service::user.auth.xboxlive.com::MBI_SSL",
        )
        .send()
        .await?
        .json()
        .await?)
}

async fn get_access_token_from_refresh_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> anyhow::Result<Value> {
    Ok(client
        .post("https://login.live.com/oauth20_token.srf")
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(
            "client_id=00000000402b5328".to_string()
                + "&grant_type=refresh_token"
                + "&refresh_token="
                + refresh_token
                + "&redirect_uri=https://login.live.com/oauth20_desktop.srf"
                + "&scope=service::user.auth.xboxlive.com::MBI_SSL",
        )
        .send()
        .await?
        .json()
        .await?)
}

struct XboxAuth {
    xbl_token: String,
    xbl_uhs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XboxAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XboxAuthBody {
    #[serde(rename = "Properties")]
    properties: XboxAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

impl XboxAuthBody {
    fn new(access_token: &str) -> Self {
        Self {
            properties: XboxAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: access_token.to_string(),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        }
    }
}

async fn xbox_authenticate(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<XboxAuth> {
    let response: Value = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(serde_json::to_string(&XboxAuthBody::new(access_token))?)
        .send()
        .await?
        .json()
        .await?;
    Ok(XboxAuth {
        xbl_token: response["Token"]
            .as_str()
            .ok_or(anyhow!("No XBL Token".to_string()))?
            .to_string(),
        xbl_uhs: response["DisplayClaims"]["xui"][0]["uhs"]
            .as_str()
            .ok_or(anyhow!("No XBL UHS"))?
            .to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XSTSAuthProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct XSTSAuthBody {
    #[serde(rename = "Properties")]
    properties: XSTSAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

impl XSTSAuthBody {
    fn new(xbl_token: &str) -> Self {
        Self {
            properties: XSTSAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_token.to_string()],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        }
    }
}

async fn xsts_authenticate(client: &reqwest::Client, xbl_token: &str) -> anyhow::Result<String> {
    let response: Value = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(serde_json::to_string(&XSTSAuthBody::new(xbl_token))?)
        .send()
        .await?
        .json()
        .await?;
    Ok(response["Token"]
        .as_str()
        .ok_or(anyhow!("No token".to_string()))?
        .to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

async fn minecraft_authenticate(
    client: &reqwest::Client,
    xbl_uhs: &str,
    xsts_token: &str,
) -> anyhow::Result<String> {
    let response: Value = client
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
    Ok(response["access_token"]
        .as_str()
        .ok_or(anyhow!("No Access Token"))?
        .to_string())
}

async fn check_game(client: &reqwest::Client, minecraft_access_token: &str) -> anyhow::Result<()> {
    let response = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {minecraft_access_token}"))
        .send()
        .await?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("Can't get game profile"))
    }
}

async fn get_player_infomations(
    client: &reqwest::Client,
    minecraft_access_token: &str,
) -> anyhow::Result<Value> {
    Ok(client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {minecraft_access_token}"))
        .send()
        .await?
        .json()
        .await?)
}

pub enum LoginMethod {
    RefreshToken,
    AccessCode,
}

/// Login or refresh login.
///
/// Note: Shouldn't save refresh token to config file
pub async fn microsoft_login(
    code_or_token: &str,
    method: LoginMethod,
) -> anyhow::Result<(String, Profile)> {
    // let client = HTTP_CLIENT.get().unwrap();
    let client = reqwest::Client::new();
    let access_token_response = match method {
        LoginMethod::RefreshToken => get_access_token_from_refresh_token(&client, code_or_token)
            .await
            .unwrap(),
        LoginMethod::AccessCode => get_access_token(&client, code_or_token).await.unwrap(),
    };
    println!("{:#?}", access_token_response);
    let access_token = access_token_response["access_token"]
        .as_str()
        .ok_or(anyhow!("No access token"))
        .unwrap()
        .to_string();
    let refresh_token = access_token_response["refresh_token"]
        .as_str()
        .ok_or(anyhow!("No refresh token"))
        .unwrap()
        .to_string();

    let xbox_auth_response = xbox_authenticate(&client, &access_token).await.unwrap();
    let xsts_token = xsts_authenticate(&client, &xbox_auth_response.xbl_token)
        .await
        .unwrap();
    let minecraft_access_token =
        minecraft_authenticate(&client, &xbox_auth_response.xbl_uhs, &xsts_token)
            .await
            .unwrap();
    check_game(&client, &minecraft_access_token).await.unwrap();
    let player_info = get_player_infomations(&client, &minecraft_access_token)
        .await
        .unwrap();
    println!("{:#?}", player_info);
    println!("{:#?}", refresh_token);
    Ok((
        refresh_token,
        Profile {
            profile_name: serde_json::from_value(player_info["name"].clone())?,
            uuid: serde_json::from_value(player_info["id"].clone())?,
            skins: serde_json::from_value(player_info["skins"].clone())?,
            capes: serde_json::from_value(player_info["capes"].clone())?,
        },
    ))
}
