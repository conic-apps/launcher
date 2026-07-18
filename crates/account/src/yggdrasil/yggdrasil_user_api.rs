use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;
use url::Url;
use uuid::Uuid;

use crate::error::*;

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
struct ProfileResponse {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    access_token: String,
    client_token: String,
    available_profiles: Vec<ProfileResponse>,
    selected_profile: Option<ProfileResponse>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YggdrasilAccount {
    pub api_root: String,
    pub account_identifier: String,
    pub access_token: String,
    pub client_token: String,
    pub profile_name: String,
    pub profile_uuid: String,
    pub added_at: u64,
}

pub async fn authenticate(
    api_root: &str,
    username: String,
    password: String,
) -> Result<AuthResponse> {
    let request_body = LoginRequest {
        username,
        password,
        agent: LoginRequestAgent {
            name: "Minecraft".to_string(),
            version: 1,
        },
    };
    let request_url = Url::parse(api_root)?.join("authserver/authenticate")?;
    Ok(HTTP_CLIENT
        .post(request_url)
        .json(&request_body)
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

pub async fn is_account_token_valid(account: YggdrasilAccount) -> Result<bool> {
    let request_body = VerifyRequest {
        access_token: account.access_token.clone(),
        client_token: account.client_token.clone(),
    };
    let request_url = Url::parse(&account.api_root)?.join("authserver/validate")?;
    let status = HTTP_CLIENT
        .get(request_url)
        .json(&request_body)
        .send()
        .await?
        .status()
        .as_u16();
    Ok(status == 204)
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

pub async fn refresh(account: YggdrasilAccount) -> Result<YggdrasilAccount> {
    let refresh_request = RefreshRequest {
        access_token: account.access_token,
        client_token: account.client_token,
        selected_profile: ProfileResponse {
            id: account.profile_uuid,
            name: account.profile_name,
        },
    };
    let request_url = Url::parse(&account.api_root)?.join("authserver/refresh")?;
    let refresh_response: RefreshResponse = HTTP_CLIENT
        .get(request_url)
        .json(&refresh_request)
        .send()
        .await?
        .json()
        .await?;
    let refreshed_account = YggdrasilAccount {
        access_token: refresh_response.access_token,
        client_token: refresh_response.client_token,
        profile_name: refresh_response.selected_profile.name,
        profile_uuid: refresh_response.selected_profile.id,
        api_root: account.api_root,
        account_identifier: account.account_identifier,
        added_at: account.added_at,
    };
    Ok(refreshed_account)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvalidateRequest {
    access_token: String,
    client_token: String,
}

pub async fn invalidate(api_root: &str, access_token: String, client_token: String) -> Result<()> {
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

#[derive(Serialize, Deserialize)]
pub struct Profile {
    id: String,
    name: String,
    properties: Vec<ProfileProperty>,
}

#[derive(Serialize, Deserialize)]
struct ProfileProperty {
    name: String,
    value: String,
}

pub async fn get_profile(api_root: &str, uuid: Uuid) -> Result<Profile> {
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
