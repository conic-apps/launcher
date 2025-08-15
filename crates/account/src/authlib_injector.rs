// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    fs::read_to_string,
    time::{SystemTime, UNIX_EPOCH},
};

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;
use url::Url;
use uuid::Uuid;

use crate::error::*;

pub async fn add_yggdrasil_server(api_root: &str) -> Result<()> {
    let parsed_api_root = normalize_url(api_root)?;
    let resolved_api_url = resolve_api_url(parsed_api_root.as_str()).await?;
    let mut servers = list_yggdrasil_server()?;
    servers.push(resolved_api_url);
    save_yggdrasil_servers(servers)?;
    Ok(())
}

/// If we don't know scheme, use https
fn normalize_url(input: &str) -> Result<Url> {
    let s = input.trim();
    let s_lower = s.to_ascii_lowercase();
    let fixed = if s_lower.starts_with("http://") || s_lower.starts_with("https://") {
        s.to_string()
    } else if s.starts_with("//") {
        format!("https:{}", s)
    } else {
        format!("https://{}", s)
    };
    Ok(Url::parse(&fixed)?)
}

pub fn delete_yggdrasil_server(index_to_delete: usize) -> Result<()> {
    let servers = list_yggdrasil_server()?;
    let mut result = vec![];
    for (index, server) in servers.iter().enumerate() {
        if index == index_to_delete {
            continue;
        }
        result.push(server.to_string());
    }
    save_yggdrasil_servers(result)?;
    Ok(())
}

fn save_yggdrasil_servers(servers: Vec<String>) -> Result<()> {
    let path = DATA_LOCATION.root.join("yggdrasil_servers.json");
    let contents = serde_json::to_string_pretty(&servers)?;
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn list_yggdrasil_server() -> Result<Vec<String>> {
    let path = DATA_LOCATION.root.join("yggdrasil_servers.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

async fn resolve_api_url(url: &str) -> Result<String> {
    let response = HTTP_CLIENT.get(url).send().await?;
    let response_headers = response.headers();
    if let Some(api_location) = response_headers.get("x-authlib-injector-api-location") {
        // TODO: If error ,show a message said
        // server response incorrect, let your
        // server owner fix it
        return Ok(api_location.to_str()?.to_string());
    };
    Ok(url.to_string())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YggdrasilServerInfo {
    /// See https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83#meta-%E4%B8%AD%E7%9A%84%E5%85%83%E6%95%B0%E6%8D%AE
    pub meta: HashMap<String, Value>,
    pub skin_domains: Vec<String>,
    pub signature_public_key: String,
}

pub async fn get_yggdrasil_server_info(api_root: &str) -> Result<YggdrasilServerInfo> {
    Ok(HTTP_CLIENT.get(api_root).send().await?.json().await?)
}

pub async fn get_yggdrasil_server_info_raw(api_root: &str) -> Result<String> {
    Ok(HTTP_CLIENT.get(api_root).send().await?.text().await?)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    username: String,
    password: String,
    agent: LoginRequestAgent,
}

#[derive(Serialize, Deserialize)]
struct LoginRequestAgent {
    name: String,
    version: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    access_token: String,
    client_token: String,
    available_profiles: Vec<ProfileResponse>,
    selected_profile: Option<ProfileResponse>,
}

#[derive(Serialize, Deserialize)]
struct ProfileResponse {
    id: String,
    name: String,
}

pub async fn login(api_root: &str, username: String, password: String) -> Result<LoginResponse> {
    let request_body = LoginRequest {
        username,
        password,
        agent: LoginRequestAgent {
            name: "Minecraft".to_string(),
            version: 1,
        },
    };
    let request_url = Url::parse(api_root)?
        .join("authserver")?
        .join("authenticate")?;
    Ok(HTTP_CLIENT
        .post(request_url)
        .json(&request_body)
        .send()
        .await?
        .json()
        .await?)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthlibInjectorAccount {
    pub api_root: String,
    pub account_identifier: String,
    pub access_token: String,
    pub client_token: String,
    pub profile_name: String,
    pub profile_uuid: String,
    pub added_at: u64,
}

pub fn add_account(account: AuthlibInjectorAccount) -> Result<()> {
    let accounts = list_accounts()?;
    let mut filtered_accounts = accounts
        .into_iter()
        .filter(|x| {
            let same = x.1.api_root == account.api_root
                && x.1.account_identifier == account.account_identifier
                && x.1.profile_uuid == account.profile_uuid;
            !same
        })
        .collect::<HashMap<_, _>>();
    filtered_accounts.insert(
        Uuid::from_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Error System Time")
                .as_nanos(),
        ),
        account,
    );
    save_accounts(filtered_accounts)?;
    Ok(())
}

pub async fn delete_account(account_key: Uuid) -> Result<()> {
    let mut accounts = list_accounts()?;
    if let Some(removed_account) = accounts.remove(&account_key) {
        invalidate_token(
            &removed_account.api_root,
            removed_account.access_token,
            removed_account.client_token,
        )
        .await?;
    };
    save_accounts(accounts)?;
    Ok(())
}

fn save_accounts(accounts: HashMap<Uuid, AuthlibInjectorAccount>) -> Result<()> {
    let path = DATA_LOCATION.root.join("accounts.authlib-injector.json");
    let contents = serde_json::to_string_pretty(&accounts)?;
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn list_accounts() -> Result<HashMap<Uuid, AuthlibInjectorAccount>> {
    let path = DATA_LOCATION.root.join("accounts.authlib-injector.json");
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn get_account(account_key: Uuid) -> Result<AuthlibInjectorAccount> {
    let accounts = list_accounts()?;
    accounts
        .get(&account_key)
        .ok_or(Error::AccountNotfound(account_key))
        .cloned()
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    id: String,
    name: String,
    properties: Vec<ProfileProperty>,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileProperty {
    name: String,
    value: String,
}

pub async fn get_profile_info(api_root: &str, uuid: Uuid) -> Result<Profile> {
    let uuid = uuid.simple().to_string();
    Ok(HTTP_CLIENT
        .get(
            Url::parse(api_root)?
                .join(&format!("sessionserver/session/minecraft/profile/{uuid}"))?,
        )
        .send()
        .await?
        .json()
        .await?)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyRequest {
    access_token: String,
    client_token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    access_token: String,
    client_token: String,
    selected_profile: ProfileResponse,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshResponse {
    access_token: String,
    client_token: String,
    selected_profile: ProfileResponse,
}

/// If failed, enter password, then invoke `relogin_account()`
pub async fn check_and_refresh_account(account_key: Uuid) -> Result<()> {
    let accounts = list_accounts()?;
    let selected_account = accounts
        .get(&account_key)
        .ok_or(Error::AccountNotfound(account_key))?
        .clone();
    let request_body = VerifyRequest {
        access_token: selected_account.access_token.clone(),
        client_token: selected_account.client_token.clone(),
    };
    let request_url = Url::parse(&selected_account.api_root)?.join("authserver/validate")?;
    let status = HTTP_CLIENT
        .get(request_url)
        .json(&request_body)
        .send()
        .await?
        .status()
        .as_u16();
    if status == 204 {
        return Ok(());
    }
    refresh_account(account_key).await?;
    Ok(())
}

pub async fn refresh_account(account_key: Uuid) -> Result<()> {
    let mut accounts = list_accounts()?;
    let selected_account = accounts
        .get(&account_key)
        .ok_or(Error::AccountNotfound(account_key))?
        .clone();

    let refresh_request = RefreshRequest {
        access_token: selected_account.access_token,
        client_token: selected_account.client_token,
        selected_profile: ProfileResponse {
            id: selected_account.profile_uuid,
            name: selected_account.profile_name,
        },
    };
    let request_url = Url::parse(&selected_account.api_root)?.join("authserver/refresh")?;
    let refresh_response: RefreshResponse = HTTP_CLIENT
        .get(request_url)
        .json(&refresh_request)
        .send()
        .await?
        .json()
        .await?;
    let new_account = AuthlibInjectorAccount {
        access_token: refresh_response.access_token,
        client_token: refresh_response.client_token,
        profile_name: refresh_response.selected_profile.name,
        profile_uuid: refresh_response.selected_profile.id,
        api_root: selected_account.api_root,
        account_identifier: selected_account.account_identifier,
        added_at: selected_account.added_at,
    };
    accounts.insert(account_key, new_account);
    save_accounts(accounts)?;
    Ok(())
}

pub async fn relogin_account(account_key: Uuid, password: String) -> Result<()> {
    let mut accounts = list_accounts()?;
    let selected_account = accounts
        .get(&account_key)
        .ok_or(Error::AccountNotfound(account_key))?
        .clone();
    let login_response = login(
        &selected_account.api_root,
        selected_account.account_identifier.clone(),
        password,
    )
    .await?;
    if let Some(selected_profile) = login_response.selected_profile {
        if selected_profile.id == selected_account.profile_uuid {
            accounts.insert(
                account_key,
                AuthlibInjectorAccount {
                    api_root: selected_account.api_root,
                    account_identifier: selected_account.account_identifier,
                    access_token: login_response.access_token,
                    client_token: login_response.client_token,
                    profile_name: selected_profile.name,
                    profile_uuid: selected_account.profile_uuid,
                    added_at: selected_account.added_at,
                },
            );
        } else {
            return Err(Error::ProfileUnavailable);
        }
    } else {
        let selected_profile_vec = login_response
            .available_profiles
            .iter()
            .filter(|x| x.id == selected_account.profile_uuid)
            .collect::<Vec<_>>();
        if let Some(selected_profile) = selected_profile_vec.first() {
            accounts.insert(
                account_key,
                AuthlibInjectorAccount {
                    api_root: selected_account.api_root,
                    account_identifier: selected_account.account_identifier,
                    access_token: login_response.access_token,
                    client_token: login_response.client_token,
                    profile_name: selected_profile.name.clone(),
                    profile_uuid: selected_account.profile_uuid,
                    added_at: selected_account.added_at,
                },
            );
        } else {
            return Err(Error::ProfileUnavailable);
        }
    }
    save_accounts(accounts)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvalidateRequest {
    access_token: String,
    client_token: String,
}

pub async fn invalidate_token(
    api_root: &str,
    access_token: String,
    client_token: String,
) -> Result<()> {
    let request_body = InvalidateRequest {
        access_token,
        client_token,
    };
    let request_url = Url::parse(api_root)?.join("authserver/invalidate")?;
    HTTP_CLIENT
        .post(request_url)
        .json(&request_body)
        .send()
        .await?;
    Ok(())
}
