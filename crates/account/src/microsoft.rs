// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::*;
use folder::DATA_LOCATION;
use shared::HTTP_CLIENT;

#[derive(Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub refresh_token: String,
    pub access_token: String,
    pub expires_on: u64,
    pub profile: Profile,
}

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

pub fn list_accounts() -> Result<Vec<MicrosoftAccount>> {
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn get_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    if !path.exists() {
        return Err(Error::AccountNotfound(uuid));
    }
    let data = std::fs::read_to_string(path)?;
    let accounts = serde_json::from_str::<Vec<MicrosoftAccount>>(&data)?;
    accounts
        .into_iter()
        .filter(|x| x.profile.uuid == uuid)
        .collect::<Vec<_>>()
        .first()
        .ok_or(Error::AccountNotfound(uuid))
        .cloned()
}

fn save_account(account: MicrosoftAccount) -> Result<()> {
    let mut accounts = list_accounts()?;
    accounts.push(account);
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    let contents = serde_json::to_string_pretty(&accounts)?;
    std::fs::write(&path, &contents)?;
    Ok(())
}

pub fn delete_account(uuid: Uuid) -> Result<()> {
    let accounts = list_accounts()?;
    let result = accounts
        .into_iter()
        .filter(|x| x.profile.uuid != uuid)
        .collect::<Vec<MicrosoftAccount>>();
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    let contents = serde_json::to_string_pretty(&result)?;
    std::fs::write(&path, &contents)?;
    Ok(())
}

pub async fn add_account(code: String) -> Result<()> {
    info!("Signing in through Microsoft");
    let account = microsoft_login(LoginPayload::AccessCode(code)).await?;
    if get_account(account.profile.uuid).is_ok() {
        error!("The account has already been added, replace it.");
        delete_account(account.profile.uuid)?;
    }
    save_account(account)?;
    Ok(())
}

/// Checks whether the given account's access token is close to expiration,
/// and refreshes it if necessary.
pub(crate) async fn check_and_refresh_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    info!("Checking account: {uuid}");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    const AHEAD: u64 = 3600 * 4;
    let account = get_account(uuid)?;
    if now > account.expires_on - AHEAD {
        info!("The access token will expire in 4 hours");
        let refreshed_account = refresh_account(account.profile.uuid).await?;
        Ok(refreshed_account)
    } else {
        info!(
            "The access token will expire in {} seconds, no need to refresh.",
            account.expires_on - now
        );
        Ok(account.clone())
    }
}

pub async fn refresh_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    info!("Start refreshing the account: {uuid}");
    let accounts = list_accounts()?;
    let mut result = vec![];
    for account in accounts {
        result.push(microsoft_login(LoginPayload::RefreshToken(account.refresh_token)).await?)
    }
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    let contents = serde_json::to_string_pretty(&result)?;
    std::fs::write(&path, &contents)?;
    Ok(result.first().ok_or(Error::AccountNotfound(uuid))?.clone())
}

// #[cfg(not(debug_assertions))]
pub async fn refresh_all_accounts() -> Result<()> {
    let accounts = list_accounts()?;
    let mut result = vec![];
    for account in accounts {
        result.push(microsoft_login(LoginPayload::RefreshToken(account.refresh_token)).await?)
    }
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    let contents = serde_json::to_string_pretty(&result)?;
    std::fs::write(&path, &contents)?;
    Ok(())
}

// #[cfg(debug_assertions)]
// pub async fn refresh_all_accounts() -> Result<()> {
//     info!("Accounts are not refreshed on app launch in debug mode.");
//     Ok(())
// }

/// Login or refresh login.
///
/// Note: Shouldn't save refresh token to config file
pub async fn microsoft_login(payload: LoginPayload) -> Result<MicrosoftAccount> {
    let access_token_response = match payload {
        LoginPayload::RefreshToken(token) => get_access_token_from_refresh_token(&token).await?,
        LoginPayload::AccessCode(code) => get_access_token(&code).await?,
    };
    let access_token = access_token_response["access_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "access_token".to_string(),
        ))?
        .to_string();
    let expires_in = access_token_response["expires_in"]
        .as_u64()
        .ok_or(Error::MicrosoftResponseMissingKey("expires_in".to_string()))?;
    let refresh_token = access_token_response["refresh_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "refresh_token".to_string(),
        ))?
        .to_string();
    info!("Successfully get Microsoft access token");

    let xbox_auth_response = xbox_authenticate(&access_token).await?;
    info!("Successfully login Xbox");

    let xsts_token = xsts_authenticate(&xbox_auth_response.xbl_token).await?;
    info!("Successfully verify XSTS");

    let minecraft_access_token =
        minecraft_authenticate(&xbox_auth_response.xbl_uhs, &xsts_token).await?;
    info!("Successfully get Minecraft access token");

    check_ownership(&minecraft_access_token).await?;
    info!("Successfully check ownership");

    let game_profile = get_game_profile(&minecraft_access_token).await?;
    info!("Successfully get game profile");

    Ok(MicrosoftAccount {
        refresh_token,
        access_token: minecraft_access_token,
        expires_on: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Incorrect System Time")
            .as_secs()
            + expires_in,
        profile: Profile {
            profile_name: serde_json::from_value(game_profile["name"].clone())?,
            uuid: serde_json::from_value(game_profile["id"].clone())?,
            skins: resolve_skins(serde_json::from_value(game_profile["skins"].clone())?).await,
            capes: serde_json::from_value(game_profile["capes"].clone())?,
        },
    })
}

async fn get_access_token(code: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
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

async fn get_access_token_from_refresh_token(refresh_token: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
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

#[derive(Clone, Serialize, Deserialize)]
struct XboxAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Clone, Serialize, Deserialize)]
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

async fn xbox_authenticate(access_token: &str) -> Result<XboxAuth> {
    let response: Value = HTTP_CLIENT
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
            .ok_or(Error::MicrosoftResponseMissingKey("xbl Token".to_string()))?
            .to_string(),
        xbl_uhs: response["DisplayClaims"]["xui"][0]["uhs"]
            .as_str()
            .ok_or(Error::MicrosoftResponseMissingKey("xui_uhs".to_string()))?
            .to_string(),
    })
}

#[derive(Clone, Serialize, Deserialize)]
struct XSTSAuthProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
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

async fn xsts_authenticate(xbl_token: &str) -> Result<String> {
    let response: Value = HTTP_CLIENT
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
        .ok_or(Error::MicrosoftResponseMissingKey("Token".to_string()))?
        .to_string())
}

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

async fn minecraft_authenticate(xbl_uhs: &str, xsts_token: &str) -> Result<String> {
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
    Ok(response["access_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "access_token".to_string(),
        ))?
        .to_string())
}

async fn check_ownership(minecraft_access_token: &str) -> Result<()> {
    let response = HTTP_CLIENT
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {minecraft_access_token}"))
        .send()
        .await?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(Error::OwnershipCheckFailed)
    }
}

async fn get_game_profile(minecraft_access_token: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
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

pub async fn relogin_account(uuid: Uuid, code: String) -> Result<()> {
    info!("Signing in through Microsoft");
    let new_account = microsoft_login(LoginPayload::AccessCode(code)).await?;

    let is_different_account = uuid != new_account.profile.uuid;
    if is_different_account {
        delete_account(uuid)?;
        save_account(new_account)?;
        return Ok(());
    };

    let accounts = list_accounts()?;
    let mut result = Vec::with_capacity(accounts.len());

    for account in accounts {
        if account.profile.uuid == new_account.profile.uuid {
            result.push(new_account.clone());
        } else {
            result.push(account)
        }
    }
    let path = DATA_LOCATION.root.join("accounts.microsoft.json");
    let contents = serde_json::to_string_pretty(&result)?;
    std::fs::write(&path, &contents)?;
    Ok(())
}
