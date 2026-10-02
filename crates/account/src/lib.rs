// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The account model and the flows that create and refresh accounts:
//! Microsoft (OAuth → Xbox Live → XSTS → Minecraft services), offline
//! profiles, and Yggdrasil (authlib-injector) servers.
//!
//! The logic lives in the modules (`offline.rs`, `microsoft/`, `yggdrasil/`),
//! and `microsoft_task` owns the at-most-one running Microsoft login task.
//!
//! `serde` renames exist only where a *wire* format requires them — the
//! `camelCase` renames on the Yggdrasil request bodies, the `textureKey` rename
//! on a Microsoft skin — because the servers and the shared `config.toml` speak
//! them. [`microsoft::LoginReporter`] reports through a closure the app points
//! at its UI thread.
//!
//! The browser flow cannot be handed a fixed redirect URI, because its loopback
//! listener (`authcode`) binds an OS-chosen port that is unknown until the flow
//! starts. [`microsoft::redeem_access_token`] therefore takes the
//! `redirect_uri` to repeat rather than carrying a literal. The refresh request
//! sends none: RFC 6749 §6 makes it conditional, and an account does not keep
//! the URI the code was issued against.
//!
//! The serialized [`Account`] form lives as `current_account` in `config.toml`,
//! and the account files as `accounts/*.json` in the data directory.

use std::path::Path;

use base64::{Engine, engine::general_purpose};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use storage::LOCATIONS;
use uuid::Uuid;

use crate::microsoft::MicrosoftAccount;
use crate::offline::OfflineAccount;
use crate::yggdrasil::YggdrasilAccount;

pub mod authcode;
mod error;
pub mod microsoft;
mod microsoft_task;
pub mod offline;
pub mod yggdrasil;

pub use error::*;
pub use microsoft_task::{LoginRequest, LoginTaskState};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Account {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
    Yggdrasil(YggdrasilAccount),
}

impl Account {
    /// The display name of the account's current profile.
    pub fn get_profile_name(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.profile_name.to_string(),
            Account::Yggdrasil(account) => account.profile.name.to_string(),
            Account::Offline(account) => account.name.to_string(),
        }
    }

    /// The profile UUID in hyphenated form.
    pub fn get_profile_uuid(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.uuid.to_string(),
            Account::Yggdrasil(account) => account.profile.id.to_string(),
            Account::Offline(account) => account.uuid.to_string(),
        }
    }

    /// The token a launch passes to the game.
    pub fn get_access_token(&self) -> String {
        match self {
            Account::Microsoft(account) => account.minecraft_access_token.to_string(),
            Account::Yggdrasil(account) => account.access_token.to_string(),
            Account::Offline(_) => "114514".to_string(),
        }
    }

    /// The `--userType` a launch passes to the game: `msa` for a Microsoft
    /// account, `mojang` for everything else.
    pub fn get_user_type(&self) -> String {
        match self {
            Account::Microsoft(_) => "msa".to_string(),
            Account::Yggdrasil(_) => "mojang".to_string(),
            Account::Offline(_) => "mojang".to_string(),
        }
    }

    /// The account type: `Microsoft`, `Offline` or `Yggdrasil` — the tag
    /// [`Account`] serializes itself under.
    pub fn kind(&self) -> &'static str {
        match self {
            Account::Microsoft(_) => "Microsoft",
            Account::Offline(_) => "Offline",
            Account::Yggdrasil(_) => "Yggdrasil",
        }
    }

    /// A stable key identifying the account: the lowercased type and the
    /// profile UUID, e.g. `microsoft-…`.
    pub fn key(&self) -> String {
        format!("{}-{}", self.kind().to_lowercase(), self.get_profile_uuid())
    }
}

/// Every stored account, grouped by kind.
#[derive(Serialize, Deserialize)]
pub struct Accounts {
    pub microsoft: Vec<MicrosoftAccount>,
    pub offline: Vec<OfflineAccount>,
    pub yggdrasil: Vec<YggdrasilAccount>,
}

/// Reads and merges all stored accounts.
///
/// Synchronous on purpose: the game view rebuilds its account list while
/// rendering, and the three account files are a few kilobytes.
pub fn list_accounts() -> Accounts {
    Accounts {
        microsoft: read_json(&LOCATIONS.launcher.accounts.join("microsoft.json")),
        offline: read_json(&LOCATIONS.launcher.accounts.join("offline.json")),
        yggdrasil: read_json(&LOCATIONS.launcher.accounts.join("yggdrasil-accounts.json")),
    }
}

/// A stored list, or an empty one when the file is missing, unreadable, or
/// malformed.
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Vec<T> {
    let Ok(data) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_else(|error| {
        log::warn!("failed to parse {}: {error}", path.display());
        Vec::new()
    })
}

/// Writes a skin handed over as a `data:` URL to `path`.
///
/// The value is a Microsoft profile's `skins[].url` — a base64 PNG, with or
/// without the `data:image/png;base64,` prefix.
pub async fn save_skin(base64_skin_url: String, path: String) -> Result<()> {
    let data = base64_skin_url
        .split_once(',')
        .map(|(_, data)| data)
        .unwrap_or(&base64_skin_url)
        .to_string();
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(&data)
        .or_else(|_| general_purpose::STANDARD.decode(data))?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

/// The UUID Minecraft derives for an offline player of `username`.
///
/// The MD5 of `OfflinePlayer:<name>` with the version/variant bits of a
/// name-based (v3) UUID set — the same value the game computes for itself, so
/// an offline profile identifies as the same player on a server.
pub fn get_uuid_from_username(username: &str) -> Uuid {
    let digest = Md5::digest(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes: [u8; 16] = digest.into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
