// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose};
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use crate::error::*;
use shared::{HTTP_CLIENT, UrlExt};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthRequest {
    username: String,
    password: String,
    agent: AuthRequestAgent,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthRequestAgent {
    name: String,
    version: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub properties: Option<Vec<ProfileProperty>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

/// The answer of `authserver/authenticate`.
///
/// The `camelCase` renames are the Yggdrasil wire format the server speaks and
/// stay. The fields are `pub` because the caller reads them directly.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub client_token: String,
    pub available_profiles: Vec<Profile>,
    pub selected_profile: Option<Profile>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Texture {
    pub url: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YggdrasilAccount {
    pub api_root: String,
    pub username: String,
    pub access_token: String,
    pub client_token: String,
    pub identifier: Uuid,
    pub profile: Profile,
    pub textures: HashMap<String, Texture>,
    pub added_at: u64,
}

pub async fn authenticate(
    api_root: &str,
    username: String,
    password: String,
) -> Result<AuthResponse> {
    // The username is logged and the password never is: this is the one place a
    // support log can say which account failed to sign in without recording the
    // secret that goes with it. Logged before the body is built, which moves both.
    log::info!("Authenticating against {api_root} as {username}");
    let request_body = AuthRequest {
        username,
        password,
        agent: AuthRequestAgent {
            name: "Minecraft".to_string(),
            version: 1,
        },
    };
    let request_url = Url::parse(api_root)?.append_path(["authserver", "authenticate"])?;
    let mut auth_response: AuthResponse = HTTP_CLIENT
        .post(request_url)
        .json(&request_body)
        .send()
        .await?
        .json()
        .await?;
    let available_profile_with_properties = get_profiles(
        api_root,
        auth_response
            .available_profiles
            .into_iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>(),
    )
    .await?;
    auth_response.available_profiles = available_profile_with_properties;
    if let Some(selected_profile) = auth_response.selected_profile {
        auth_response.selected_profile = Some(get_profile(api_root, selected_profile.id).await?);
    }
    let futures = auth_response
        .available_profiles
        .into_iter()
        .map(find_and_replace_textures_property)
        .collect::<Vec<_>>();
    auth_response.available_profiles = stream::iter(futures).buffer_unordered(4).collect().await;
    auth_response.selected_profile = match auth_response.selected_profile {
        Some(profile) => Some(find_and_replace_textures_property(profile).await),
        None => None,
    };
    auth_response
        .available_profiles
        .sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Ok(auth_response)
}

async fn find_and_replace_textures_property(profile: Profile) -> Profile {
    if profile.properties.is_none() {
        return profile;
    }
    #[allow(clippy::unwrap_used)]
    let futures = profile
        .properties
        .unwrap()
        .into_iter()
        .map(async |property| {
            if property.name != "textures" {
                property
            } else {
                ProfileProperty {
                    name: property.name,
                    value: replace_textures_property_value(property.value).await,
                    signature: property.signature,
                }
            }
        })
        .collect::<Vec<_>>();
    let replaced_properties = futures::stream::iter(futures)
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
    Profile {
        name: profile.name,
        id: profile.id,
        properties: Some(replaced_properties),
    }
}

async fn replace_textures_property_value(value: String) -> String {
    // Every one of these used to fall back silently, and the last one is the
    // dangerous one: a failure at `to_string` produced an *empty* base64 string
    // that was then handed to the game as the account's texture property. The
    // property decoding is therefore replaced with an empty one rather than the
    // URL the server sent, and the game shows a blank skin for it.
    let textures_property_byte = match general_purpose::STANDARD.decode(&value) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::debug!("a textures property was not base64 ({error}); leaving it alone");
            Vec::new()
        }
    };
    let mut textures_property: Value = match serde_json::from_slice(&textures_property_byte) {
        Ok(property) => property,
        Err(error) => {
            log::debug!("a textures property was not JSON ({error}); leaving it alone");
            Value::Null
        }
    };
    let textures = match parse_textures(value.clone()).await {
        Ok(textures) => textures,
        Err(error) => {
            log::debug!("could not resolve a profile's textures ({error}); keeping the URLs");
            HashMap::new()
        }
    };
    textures_property["textures"] = {
        #[allow(clippy::unwrap_used)]
        serde_json::to_value(textures).unwrap()
    };
    // The one that mattered: a serialization failure here used to yield `""`, and
    // that empty string was the texture property the game was handed — a blank
    // skin with nothing to explain it. The original property is returned instead,
    // which is at worst the raw URLs and at best identical.
    match serde_json::to_string(&textures_property) {
        Ok(serialized) => general_purpose::STANDARD.encode(serialized),
        Err(error) => {
            log::debug!(
                "could not re-encode the replaced textures property ({error}); keeping the \\
                 server's own value"
            );
            value
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyRequest {
    access_token: String,
    client_token: String,
}

pub async fn validate(account: YggdrasilAccount) -> Result<bool> {
    let request_body = VerifyRequest {
        access_token: account.access_token.clone(),
        client_token: account.client_token.clone(),
    };
    let request_url = Url::parse(&account.api_root)?.append_path(["authserver", "validate"])?;
    let status = HTTP_CLIENT
        .get(request_url)
        .json(&request_body)
        .send()
        .await?
        .status()
        .as_u16();
    // `204` is the documented "still good"; anything else is a real status worth
    // naming, and the caller only ever sees a boolean.
    if status != 204 {
        log::debug!(
            "the Yggdrasil server {} answered {status} to a session validation, which is \\
             not the documented 204",
            account.api_root
        );
    }
    Ok(status == 204)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    access_token: String,
    client_token: String,
    selected_profile: Option<Profile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshResponse {
    access_token: String,
    client_token: String,
    selected_profile: Option<Profile>,
}

pub async fn refresh(account: YggdrasilAccount) -> Result<YggdrasilAccount> {
    let refresh_request = RefreshRequest {
        access_token: account.access_token,
        client_token: account.client_token,
        selected_profile: Some(Profile {
            id: account.profile.id,
            name: account.profile.name.clone(),
            properties: None,
        }),
    };
    let request_url = Url::parse(&account.api_root)?.append_path(["authserver", "refresh"])?;
    let refresh_response: RefreshResponse = HTTP_CLIENT
        .get(request_url)
        .json(&refresh_request)
        .send()
        .await?
        .json()
        .await?;
    let profile = get_profile(
        &account.api_root,
        refresh_response
            .selected_profile
            .unwrap_or(account.profile)
            .id,
    )
    .await?;
    let textures = {
        if let Some(properties) = &profile.properties
            && let Some(property) = properties
                .iter()
                .find(|property| property.name == "textures")
        {
            parse_textures(property.value.clone()).await?
        } else {
            HashMap::new()
        }
    };
    let refreshed_account = YggdrasilAccount {
        access_token: refresh_response.access_token,
        client_token: refresh_response.client_token,
        textures,
        profile,
        identifier: account.identifier,
        api_root: account.api_root,
        username: account.username,
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
    let request_url = Url::parse(api_root)?.append_path(["authserver", "invalidate"])?;
    let status = HTTP_CLIENT
        .post(request_url)
        .json(&request_body)
        .send()
        .await?
        .status();
    // Logout is best-effort by design and the account is removed locally either
    // way, so a refusal is a warning rather than an error the caller acts on —
    // but it means the server still holds a live token, so it is recorded.
    if !status.is_success() {
        log::warn!("the Yggdrasil server {api_root} answered {status} to the session invalidation");
    }
    Ok(())
}

pub async fn get_profile(api_root: &str, uuid: Uuid) -> Result<Profile> {
    Ok(HTTP_CLIENT
        .get(Url::parse(api_root)?.append_path([
            "sessionserver",
            "session",
            "minecraft",
            "profile",
            &uuid.simple().to_string(),
        ])?)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_profiles(api_root: &str, uuids: Vec<Uuid>) -> Result<Vec<Profile>> {
    let base_url = Url::parse(api_root)?;
    let urls: Vec<Url> = uuids
        .iter()
        .map(|uuid| {
            base_url
                .clone()
                .append_path([
                    "sessionserver",
                    "session",
                    "minecraft",
                    "profile",
                    &uuid.simple().to_string(),
                ])
                .map_err(Error::InvalidBaseUrl)
        })
        .collect::<Result<_>>()?;
    let futures = urls
        .into_iter()
        .map(|url| async move { HTTP_CLIENT.get(url).send().await?.json::<Profile>().await });
    let results: Vec<_> = stream::iter(futures).buffer_unordered(10).collect().await;
    let mut profiles = Vec::with_capacity(results.len());
    for result in results {
        profiles.push(result?);
    }
    Ok(profiles)
}

async fn parse_textures(profile_property_value: String) -> Result<HashMap<String, Texture>> {
    let texture_property_base64 = profile_property_value;
    let texture_property_byte = general_purpose::STANDARD.decode(texture_property_base64)?;
    let texture_property: Value = serde_json::from_slice(&texture_property_byte)?;
    let textures: HashMap<String, Texture> =
        serde_json::from_value(texture_property["textures"].clone())?;
    let futures: Vec<_> = textures
        .into_iter()
        .map(|(model_type, texture)| resolve_texture(model_type, texture))
        .collect();
    let resolved_textures = stream::iter(futures)
        .buffer_unordered(4)
        .collect::<HashMap<_, _>>()
        .await;
    Ok(resolved_textures)
}

async fn resolve_texture(model_type: String, texture: Texture) -> (String, Texture) {
    (
        model_type,
        Texture {
            url: download_texture(&texture.url).await,
            metadata: texture.metadata,
        },
    )
}

async fn download_texture(url: &str) -> String {
    async fn download(url: &str) -> Result<Vec<u8>> {
        Ok(HTTP_CLIENT.get(url).send().await?.bytes().await?.to_vec())
    }
    if let Ok(content) = download(url).await {
        format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD_NO_PAD.encode(content)
        )
    } else {
        url.to_string()
    }
}
