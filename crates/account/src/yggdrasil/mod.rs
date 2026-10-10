// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use base64::{Engine, engine::general_purpose};
use serde_json::Value;
use storage::LOCATIONS;
use uuid::Uuid;

use crate::error::*;

pub mod yggdrasil_server;
pub mod yggdrasil_user_api;

pub use yggdrasil_user_api::YggdrasilAccount;

use yggdrasil_user_api::Profile;

pub async fn add_account(account: YggdrasilAccount) -> Result<()> {
    let mut accounts = list_accounts()
        .await?
        .into_iter()
        .filter(|x| {
            !(x.api_root == account.api_root
                && x.profile.name == account.profile.name
                && x.profile.id == account.profile.id)
        })
        .collect::<Vec<_>>();
    accounts.push(account);
    save_accounts(accounts).await?;
    Ok(())
}

pub async fn delete_account(account: YggdrasilAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .filter(|x| x.identifier != account.identifier)
        .collect::<Vec<_>>();
    // The server is told to drop the session, and the account is dropped locally
    // either way — so a swallowed failure here leaves a live access token on the
    // server with nothing on record that logout was ever attempted.
    if let Err(error) = yggdrasil_user_api::invalidate(
        &account.api_root,
        account.access_token,
        account.client_token,
    )
    .await
    {
        log::warn!(
            "could not invalidate the session for {} at {}: {error}; it is removed locally \
             regardless",
            account.identifier,
            account.api_root
        );
    }
    save_accounts(result).await?;
    log::info!(
        "Removed the Yggdrasil account {} from {}",
        account.identifier,
        account.api_root
    );
    Ok(())
}

async fn save_accounts(accounts: Vec<YggdrasilAccount>) -> Result<()> {
    let yggdrasil_accounts_list_file = LOCATIONS.launcher.accounts.join("yggdrasil-accounts.json");
    tokio::fs::create_dir_all(&LOCATIONS.launcher.accounts).await?;
    let serialized_yggdrasil_accounts_list = serde_json::to_string_pretty(&accounts)?;
    tokio::fs::write(
        yggdrasil_accounts_list_file,
        serialized_yggdrasil_accounts_list,
    )
    .await?;
    Ok(())
}

pub async fn list_accounts() -> Result<Vec<YggdrasilAccount>> {
    let yggdrasil_accounts_list_file = LOCATIONS.launcher.accounts.join("yggdrasil-accounts.json");
    tokio::fs::create_dir_all(&LOCATIONS.launcher.accounts).await?;
    if !yggdrasil_accounts_list_file.exists() {
        return Ok(vec![]);
    }
    // Both of these read as "no accounts" without a word, and the next add
    // writes the file back with one entry in it — a corrupt file here silently
    // costs the user every Yggdrasil account they had.
    let serialized_yggdrasil_accounts_list =
        match tokio::fs::read_to_string(&yggdrasil_accounts_list_file).await {
            Ok(contents) => contents,
            Err(error) => {
                log::warn!(
                    "Could not read {} ({error}); treating it as no Yggdrasil accounts, and \
                     adding one will overwrite it",
                    yggdrasil_accounts_list_file.display()
                );
                return Ok(vec![]);
            }
        };
    match serde_json::from_str(&serialized_yggdrasil_accounts_list) {
        Ok(accounts) => Ok(accounts),
        Err(error) => {
            log::warn!(
                "{} is not valid account json ({error}); treating it as no Yggdrasil \
                 accounts, and adding one will overwrite it",
                yggdrasil_accounts_list_file.display()
            );
            Ok(Vec::new())
        }
    }
}

pub async fn get_account(account_identifier: Uuid) -> Result<YggdrasilAccount> {
    let accounts = list_accounts().await?;
    accounts
        .into_iter()
        .find(|account| account.identifier == account_identifier)
        .ok_or(Error::AccountNotfound(account_identifier))
}

pub async fn update_account(account_identifier: Uuid, account: YggdrasilAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .map(|x| {
            if x.identifier == account_identifier {
                account.clone()
            } else {
                x
            }
        })
        .collect::<Vec<_>>();
    save_accounts(result).await?;
    Ok(())
}

/// The skin URL a profile's `textures` property carries.
///
/// The property is the base64 of a JSON document whose `textures.SKIN.url` is
/// the texture. Anything malformed reads as "no skin".
pub fn get_skin_url(profile: &Profile) -> Option<String> {
    texture_url(profile, "SKIN")
}

/// The cape URL a profile's `textures` property carries.
pub fn get_cape_url(profile: &Profile) -> Option<String> {
    texture_url(profile, "CAPE")
}

fn texture_url(profile: &Profile, model: &str) -> Option<String> {
    let property = profile
        .properties
        .as_ref()?
        .iter()
        .find(|property| property.name == "textures")?;
    // Servers may leave off the padding or wrap the value with line breaks,
    // so whitespace is stripped and both padded and unpadded decoding are
    // tried.
    let compact: String = property
        .value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    let decoded = general_purpose::STANDARD
        .decode(&compact)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(&compact))
        .ok()?;
    let document: Value = serde_json::from_slice(&decoded).ok()?;
    document["textures"][model]["url"]
        .as_str()
        .map(str::to_string)
}
