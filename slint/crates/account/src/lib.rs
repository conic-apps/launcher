// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/account`: the account data model and the
//! on-disk listing. The original crate is a Tauri plugin whose command layer
//! performs the OAuth/Yggdrasil network flows; the Slint app only needs to read
//! the accounts the user already added (and remember the selected one), so this
//! crate keeps the same serialized types and `list_accounts` reads the same
//! `accounts/*.json` files under [`slint_folder::DATA_LOCATION`].

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use slint_folder::DATA_LOCATION;
use uuid::Uuid;

/// A skin entry of a Microsoft (Mojang) profile.
#[derive(Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub state: String,
    #[serde(rename = "textureKey")]
    pub texture_key: String,
    pub url: String,
    pub variant: String,
}

/// A cape entry of a Microsoft (Mojang) profile.
#[derive(Clone, Serialize, Deserialize)]
pub struct Cape {
    pub alias: String,
    pub id: String,
    pub state: String,
    pub url: String,
}

/// The Minecraft profile attached to a Microsoft account.
#[derive(Clone, Serialize, Deserialize)]
pub struct MicrosoftProfile {
    pub profile_name: String,
    pub uuid: Uuid,
    #[serde(default)]
    pub skins: Vec<Skin>,
    #[serde(default)]
    pub capes: Vec<Cape>,
}

/// An account authenticated through Microsoft / Xbox Live.
#[derive(Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub refresh_token: String,
    pub minecraft_access_token: String,
    pub expires_at: u64,
    pub profile: MicrosoftProfile,
}

/// A locally created offline account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub name: String,
    pub uuid: Uuid,
    pub skin: Option<String>,
}

/// A profile property (e.g. the base64 `textures` blob).
#[derive(Clone, Serialize, Deserialize)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

/// An Authlib-injector / Yggdrasil profile.
#[derive(Clone, Serialize, Deserialize)]
pub struct YggdrasilProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Option<Vec<ProfileProperty>>,
}

/// A texture fetched from a Yggdrasil server.
#[derive(Clone, Serialize, Deserialize)]
pub struct Texture {
    pub url: String,
    pub metadata: Option<HashMap<String, String>>,
}

/// An account authenticated against a Yggdrasil (authlib-injector) server.
#[derive(Clone, Serialize, Deserialize)]
pub struct YggdrasilAccount {
    pub api_root: String,
    pub username: String,
    pub access_token: String,
    pub client_token: String,
    pub identifier: Uuid,
    pub profile: YggdrasilProfile,
    #[serde(default)]
    pub textures: HashMap<String, Texture>,
    pub added_at: u64,
}

/// A stored account. Serialized form matches the Tauri `account` crate
/// (`{ type, data }`) so both frontends share the same `config.toml`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Account {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
    Yggdrasil(YggdrasilAccount),
}

impl Account {
    /// The display name of the account's current profile.
    pub fn profile_name(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.profile_name.clone(),
            Account::Offline(account) => account.name.clone(),
            Account::Yggdrasil(account) => account.profile.name.clone(),
        }
    }

    /// The profile UUID.
    pub fn profile_uuid(&self) -> Uuid {
        match self {
            Account::Microsoft(account) => account.profile.uuid,
            Account::Offline(account) => account.uuid,
            Account::Yggdrasil(account) => account.profile.id,
        }
    }

    /// The account type tag (`Microsoft` / `Offline` / `Yggdrasil`).
    pub fn kind(&self) -> &'static str {
        match self {
            Account::Microsoft(_) => "Microsoft",
            Account::Offline(_) => "Offline",
            Account::Yggdrasil(_) => "Yggdrasil",
        }
    }

    /// A stable key identifying the account, matching the Vue frontend.
    pub fn key(&self) -> String {
        format!("{}-{}", self.kind().to_lowercase(), self.profile_uuid())
    }
}

/// Every stored account, grouped by kind.
#[derive(Default)]
pub struct Accounts {
    pub microsoft: Vec<MicrosoftAccount>,
    pub offline: Vec<OfflineAccount>,
    pub yggdrasil: Vec<YggdrasilAccount>,
}

/// Reads and merges all stored accounts (Microsoft, offline, Yggdrasil).
pub fn list_accounts() -> Accounts {
    Accounts {
        microsoft: read_json(&DATA_LOCATION.accounts.join("microsoft.json")),
        offline: read_json(&DATA_LOCATION.accounts.join("offline.json")),
        yggdrasil: read_json(&DATA_LOCATION.accounts.join("yggdrasil-accounts.json")),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Vec<T> {
    let Ok(data) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_else(|error| {
        log::warn!("failed to parse {}: {error}", path.display());
        Vec::new()
    })
}
