// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/account`: the account model, and the flows
//! that create and refresh accounts — Microsoft (OAuth → Xbox Live → XSTS →
//! Minecraft services), offline profiles, and Yggdrasil (authlib-injector)
//! servers.
//!
//! The original is a Tauri plugin whose logic lives in its modules
//! (`offline.rs`, `microsoft/`, `yggdrasil/`); `offline_commands.rs`,
//! `microsoft_commands.rs` and `yggdrasil_commands.rs` only put an IPC surface
//! in front of that logic. This mirror keeps the module tree, so the two
//! crates can be diffed against each other, with these differences:
//!
//!   * the command modules are gone. Two of them were pure pass-throughs to
//!     functions that are `pub` already; the third, which owns the at-most-one
//!     running login task, becomes [`microsoft_task`];
//!   * `microsoft::LoginReporter` reports through a closure instead of a
//!     `tauri::ipc::Channel`, and the `serde` derives the original carries for
//!     the IPC boundary are dropped (`slint-java-runtime`'s rule). The
//!     attributes that describe a *wire* format — the `camelCase` renames on
//!     the Yggdrasil request bodies, the `textureKey` rename on a Microsoft
//!     skin — stay, because the servers and the shared `config.toml` still
//!     speak them;
//!   * `shared::HTTP_CLIENT` and `shared::UrlExt` are inlined in [`shared`],
//!     the way `slint-install` inlines the client.
//!
//! The only other difference is mechanical: the original's private
//! `save_accounts(&Vec<Account>)` helpers take a slice here, which is what
//! clippy asks for and changes nothing about what is written.
//!
//! Structures, request bodies and on-disk formats match `crates/account/src`,
//! and so does the serialized form of [`Account`], so both frontends share the
//! same `~/.conic[-debug]/accounts/*.json` and the same `current_account` in
//! `config.toml`.

use std::path::Path;

use base64::{Engine, engine::general_purpose};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use slint_folder::DATA_LOCATION;
use uuid::Uuid;

use crate::microsoft::MicrosoftAccount;
use crate::offline::OfflineAccount;
use crate::yggdrasil::YggdrasilAccount;

mod error;
pub mod microsoft;
mod microsoft_task;
pub mod offline;
pub mod shared;
pub mod yggdrasil;

pub use error::*;
pub use microsoft_task::LoginTaskState;
pub use shared::set_system_proxy;

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Account {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
    Yggdrasil(YggdrasilAccount),
}

impl Account {
    /// The display name of the account's current profile
    /// (`crates/account/src/lib.rs`).
    pub fn get_profile_name(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.profile_name.to_string(),
            Account::Yggdrasil(account) => account.profile.name.to_string(),
            Account::Offline(account) => account.name.to_string(),
        }
    }

    /// The profile UUID, as the original returns it — the hyphenated form
    /// (`crates/account/src/lib.rs`).
    pub fn get_profile_uuid(&self) -> String {
        match self {
            Account::Microsoft(account) => account.profile.uuid.to_string(),
            Account::Yggdrasil(account) => account.profile.id.to_string(),
            Account::Offline(account) => account.uuid.to_string(),
        }
    }

    /// The token a launch passes to the game (`crates/account/src/lib.rs`).
    pub fn get_access_token(&self) -> String {
        match self {
            Account::Microsoft(account) => account.minecraft_access_token.to_string(),
            Account::Yggdrasil(account) => account.access_token.to_string(),
            Account::Offline(_) => "114514".to_string(),
        }
    }

    /// The `--userType` a launch passes to the game: `msa` for a Microsoft
    /// account, `mojang` for everything else (`crates/account/src/lib.rs`).
    pub fn get_user_type(&self) -> String {
        match self {
            Account::Microsoft(_) => "msa".to_string(),
            Account::Yggdrasil(_) => "mojang".to_string(),
            Account::Offline(_) => "mojang".to_string(),
        }
    }

    /// The account type as the frontend names it: `Microsoft`, `Offline` or
    /// `Yggdrasil` — the tag [`Account`] serializes itself under.
    pub fn kind(&self) -> &'static str {
        match self {
            Account::Microsoft(_) => "Microsoft",
            Account::Offline(_) => "Offline",
            Account::Yggdrasil(_) => "Yggdrasil",
        }
    }

    /// A stable key identifying the account, matching the Vue frontend's
    /// (`${type.toLowerCase()}-${uuid}`).
    pub fn key(&self) -> String {
        format!("{}-{}", self.kind().to_lowercase(), self.get_profile_uuid())
    }
}

/// Every stored account, grouped by kind. `cmd_list_accounts` of the original,
/// whose answer the frontend's `Accounts` type mirrors.
#[derive(Serialize, Deserialize)]
pub struct Accounts {
    pub microsoft: Vec<MicrosoftAccount>,
    pub offline: Vec<OfflineAccount>,
    pub yggdrasil: Vec<YggdrasilAccount>,
}

/// Reads and merges all stored accounts (`cmd_list_accounts`).
///
/// The one place the mirror is not `async`: the original's command awaits
/// three async readers because a Tauri command returns a future, but the Slint
/// app's game view rebuilds its account list synchronously, from the same
/// files and with the same lenient parsing the read-only `slint-account` it
/// replaces used. The three account files are a few kilobytes.
pub fn list_accounts() -> Accounts {
    Accounts {
        microsoft: read_json(&DATA_LOCATION.accounts.join("microsoft.json")),
        offline: read_json(&DATA_LOCATION.accounts.join("offline.json")),
        yggdrasil: read_json(&DATA_LOCATION.accounts.join("yggdrasil-accounts.json")),
    }
}

/// A stored list, or an empty one when the file is missing or unreadable —
/// the `unwrap_or_default` every reader in the crate applies.
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Vec<T> {
    let Ok(data) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_else(|error| {
        log::warn!("failed to parse {}: {error}", path.display());
        Vec::new()
    })
}

/// Writes a skin handed over as a `data:` URL to `path` (`cmd_save_skin`).
///
/// The value is what a Microsoft profile's `skins[].url` holds — a
/// base64 PNG, with or without the `data:image/png;base64,` prefix.
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
/// The frontend's `getUuidFromUsername` (`crates/account/index.ts`): the MD5 of
/// `OfflinePlayer:<name>` with the version/variant bits of a name-based (v3)
/// UUID set, which is what the game computes for itself — so an offline
/// profile identifies as the same player on a server.
pub fn get_uuid_from_username(username: &str) -> Uuid {
    let digest = Md5::digest(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes: [u8; 16] = digest.into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
