// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use base64::{engine::general_purpose, Engine};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Emitter;
use tauri_plugin_http::reqwest;

use crate::{DATA_LOCATION, HTTP_CLIENT, MAIN_WINDOW};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Skin {
    pub id: String,
    pub state: String,
    #[serde(rename(serialize = "textureKey", deserialize = "textureKey"))]
    pub texture_key: String,
    pub url: String,
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Cape {
    pub alias: String,
    pub id: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Profile {
    pub profile_name: String,
    pub uuid: String,
    pub skins: Vec<Skin>,
    pub capes: Vec<Cape>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum AccountType {
    Microsoft,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Account {
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub token_deadline: Option<u64>,
    pub profile: Profile,
    pub account_type: AccountType,
}

#[tauri::command]
pub fn get_accounts() -> Result<Vec<Account>, ()> {
    let path = DATA_LOCATION.get().unwrap().root.join("accounts.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(path).unwrap();
    Ok(serde_json::from_str::<Vec<Account>>(&data).unwrap())
}

fn add_account(account: Account) -> anyhow::Result<()> {
    let mut accounts = get_accounts().unwrap();
    accounts.push(account);
    let path = DATA_LOCATION.get().unwrap().root.join("accounts.json");
    let contents = serde_json::to_string_pretty(&accounts).unwrap();
    std::fs::write(&path, &contents).unwrap();
    println!("{:#?}", path);
    MAIN_WINDOW
        .get()
        .unwrap()
        .emit("refresh_accounts_list", "")
        .unwrap();
    Ok(())
}

#[tauri::command(async)]
pub async fn delete_account(uuid: String) {
    let accounts = get_accounts().unwrap();
    let result = accounts
        .into_iter()
        .filter(|x| x.profile.uuid != uuid)
        .collect::<Vec<Account>>();
    let path = DATA_LOCATION.get().unwrap().root.join("accounts.json");
    let contents = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(&path, &contents).unwrap();
    MAIN_WINDOW
        .get()
        .unwrap()
        .emit("refresh_accounts_list", "")
        .unwrap();
}

#[tauri::command(async)]
/// A command to add a microsoft account
pub async fn add_microsoft_account(code: String) -> std::result::Result<(), ()> {
    async fn add_microsoft_account(code: String) -> anyhow::Result<()> {
        info!("Signing in through Microsoft");
        let account = microsoft_login(LoginPayload::AccessCode(code)).await?;
        println!("{:#?}", account);
        add_account(account)?;
        Ok(())
    }
    match add_microsoft_account(code).await {
        anyhow::Result::Ok(x) => Ok(x),
        anyhow::Result::Err(_) => Err(()),
    }
}

#[tauri::command]
pub fn add_offline_account(_username: String) -> std::result::Result<(), ()> {
    Ok(())
}

#[tauri::command(async)]
pub async fn refresh_microsoft_account_by_uuid(uuid: String) {
    let accounts = get_accounts().unwrap();
    let mut result = vec![];
    for account in accounts {
        if account.profile.uuid != uuid
            || account.refresh_token.is_none()
            || account.account_type != AccountType::Microsoft
        {
            result.push(account)
        } else {
            result.push(
                microsoft_login(LoginPayload::RefreshToken(
                    account.refresh_token.unwrap_or_default(),
                ))
                .await
                .unwrap(),
            )
        }
    }
    let path = DATA_LOCATION.get().unwrap().root.join("accounts.json");
    let contents = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(&path, &contents).unwrap();
    MAIN_WINDOW
        .get()
        .unwrap()
        .emit("refresh_accounts_list", "")
        .unwrap();
}

#[tauri::command(async)]
pub async fn refresh_all_microsoft_account() {
    let accounts = get_accounts().unwrap();
    let mut result = vec![];
    for account in accounts {
        if account.refresh_token.is_none() || account.account_type != AccountType::Microsoft {
            result.push(account)
        } else {
            result.push(
                microsoft_login(LoginPayload::RefreshToken(
                    account.refresh_token.unwrap_or_default(),
                ))
                .await
                .unwrap(),
            )
        }
    }
    let path = DATA_LOCATION.get().unwrap().root.join("accounts.json");
    let contents = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(&path, &contents).unwrap();
    MAIN_WINDOW
        .get()
        .unwrap()
        .emit("refresh_accounts_list", "")
        .unwrap();
}

/// Login or refresh login.
///
/// Note: Shouldn't save refresh token to config file
pub async fn microsoft_login(payload: LoginPayload) -> anyhow::Result<Account> {
    // let client = HTTP_CLIENT.get().unwrap();
    let client = reqwest::Client::new();
    let access_token_response = match payload {
        LoginPayload::RefreshToken(token) => get_access_token_from_refresh_token(&client, &token)
            .await
            .unwrap(),
        LoginPayload::AccessCode(code) => get_access_token(&client, &code).await.unwrap(),
    };
    info!("Successfully get Microsoft access token");
    let access_token = access_token_response["access_token"]
        .as_str()
        .ok_or(anyhow!("No access token"))
        .unwrap()
        .to_string();
    let expires_in = access_token_response["expires_in"]
        .as_u64()
        .ok_or(anyhow!("No expires_in"))
        .unwrap();
    let refresh_token = access_token_response["refresh_token"]
        .as_str()
        .ok_or(anyhow!("No refresh token"))
        .unwrap()
        .to_string();

    let xbox_auth_response = xbox_authenticate(&client, &access_token).await.unwrap();
    info!("Successfully login Xbox");
    let xsts_token = xsts_authenticate(&client, &xbox_auth_response.xbl_token)
        .await
        .unwrap();
    info!("Successfully verify XSTS");
    let minecraft_access_token =
        minecraft_authenticate(&client, &xbox_auth_response.xbl_uhs, &xsts_token)
            .await
            .unwrap();
    info!("Successfully get Minecraft access token");
    check_game(&client, &minecraft_access_token).await.unwrap();
    info!("Successfully check ownership");
    let player_info = get_player_infomations(&client, &minecraft_access_token)
        .await
        .unwrap();
    info!("Successfully get game profile");
    Ok(Account {
        refresh_token: Some(refresh_token),
        access_token: Some(access_token),
        token_deadline: Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + expires_in),
        profile: Profile {
            profile_name: serde_json::from_value(player_info["name"].clone())?,
            uuid: serde_json::from_value(player_info["id"].clone())?,
            skins: resolve_skins(serde_json::from_value(player_info["skins"].clone())?).await,
            capes: serde_json::from_value(player_info["capes"].clone())?,
        },
        account_type: AccountType::Microsoft,
    })
}

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

pub enum LoginPayload {
    RefreshToken(String),
    AccessCode(String),
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
    async fn download_skin(url: &str) -> anyhow::Result<Vec<u8>> {
        Ok(HTTP_CLIENT
            .get()
            .unwrap()
            .get(url)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec())
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
