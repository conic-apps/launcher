// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use storage::LOCATIONS;
use uuid::Uuid;

use crate::{error::*, microsoft::account_profile_step::Profile};

mod account_profile_step;
pub mod device_code;
mod login_task;
mod microsoft_auth_step;
mod minecraft_auth_step;
mod minecraft_profile_step;
mod xbox_auth_step;
mod xsts_auth_step;

pub use login_task::{LoginEvent, LoginReporter};
pub(crate) use login_task::{login_with_auth_code, login_with_device_code};
pub use microsoft_auth_step::redeem_access_token;

/// Decodes one Microsoft endpoint's answer, naming the endpoint when it refuses.
///
/// None of these endpoints were status-checked before. A `401` body is perfectly
/// good JSON, so it parsed and then failed on a missing `Token` key, and surfaced
/// as `MicrosoftResponseMissingKey` — which reads like a schema bug rather than
/// an authentication refusal. XSTS answering `401` is in fact the ordinary
/// outcome for an account with no Xbox profile, and it is the single most
/// misdiagnosed failure in the whole chain.
///
/// The body is kept in the error and the log because the endpoints explain
/// themselves in it: XSTS names the problem in `XErr`, and the token endpoints
/// name the failing grant in `error_description`. It is truncated first — an
/// error page from a proxy in front of the endpoint can be arbitrarily long.
pub(super) async fn decode(response: reqwest::Response, endpoint: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let body: String = body.chars().take(512).collect();
        log::warn!("{endpoint} answered {status}: {body}");
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    Ok(response.json().await?)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub refresh_token: String,
    pub minecraft_access_token: String,
    pub expires_at: u64,
    pub profile: Profile,
}

pub async fn list_accounts() -> Result<Vec<MicrosoftAccount>> {
    let accounts_list_file = LOCATIONS.launcher.accounts.join("microsoft.json");
    tokio::fs::create_dir_all(&LOCATIONS.launcher.accounts).await?;
    if !accounts_list_file.exists() {
        return Ok(vec![]);
    }
    // Both of these used to become an empty list without a word, which is the
    // worst outcome this file has: `add_account` reads the list, appends one
    // entry and writes it back — so a corrupt or unreadable file reads as "no
    // accounts" and the very next sign-in overwrites it with a single element.
    // The user's other accounts are gone, and nothing said so.
    let serialized_account_list = match tokio::fs::read_to_string(&accounts_list_file).await {
        Ok(contents) => contents,
        Err(error) => {
            log::warn!(
                "Could not read {} ({error}); treating it as no Microsoft accounts, and \
                 signing in will overwrite it",
                accounts_list_file.display()
            );
            return Ok(vec![]);
        }
    };
    match serde_json::from_str(&serialized_account_list) {
        Ok(accounts) => Ok(accounts),
        Err(error) => {
            log::warn!(
                "{} is not valid account json ({error}); treating it as no Microsoft \
                 accounts, and signing in will overwrite it",
                accounts_list_file.display()
            );
            Ok(Vec::new())
        }
    }
}

pub async fn get_account(uuid: Uuid) -> Result<MicrosoftAccount> {
    let accounts = list_accounts().await?;
    accounts
        .into_iter()
        .filter(|x| x.profile.uuid == uuid)
        .collect::<Vec<_>>()
        .first()
        .ok_or(Error::AccountNotfound(uuid))
        .cloned()
}

pub async fn add_account(account: MicrosoftAccount) -> Result<()> {
    let mut accounts = list_accounts()
        .await?
        .into_iter()
        .filter(|x| x.profile.uuid != account.profile.uuid)
        .collect::<Vec<_>>();
    accounts.push(account);
    save_accounts(&accounts).await?;
    Ok(())
}

pub async fn delete_account(uuid: Uuid) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .filter(|x| x.profile.uuid != uuid)
        .collect::<Vec<MicrosoftAccount>>();
    save_accounts(&result).await?;
    Ok(())
}

pub async fn update_account(uuid: Uuid, account: &MicrosoftAccount) -> Result<()> {
    let accounts = list_accounts().await?;
    let result = accounts
        .into_iter()
        .map(|item| {
            if account.profile.uuid == uuid {
                account.clone()
            } else {
                item
            }
        })
        .collect::<Vec<MicrosoftAccount>>();
    save_accounts(&result).await?;
    Ok(())
}

pub async fn refresh_account(uuid: Uuid, force_refresh: bool) -> Result<MicrosoftAccount> {
    let account = get_account(uuid).await?;
    if !force_refresh {
        info!("Checking account: {uuid}");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Incorrect System Time")
            .as_secs();
        const AHEAD: u64 = 3600 * 4;
        if now <= account.expires_at - AHEAD {
            info!(
                "The access token will expire in {} seconds, no need to refresh.",
                account.expires_at - now
            );
            return Ok(account.clone());
        }
    }
    info!("Start refreshing the account: {uuid}");
    let (access_token, refresh_token) = {
        let tokens =
            microsoft_auth_step::get_access_token_from_refresh_token(&account.refresh_token)
                .await
                .inspect_err(|error| {
                    // Named on its own: `invalid_grant` here means the user revoked
                    // the app, and it is the only place that says so.
                    log::error!("Could not exchange the refresh token for {uuid}: {error}");
                })?;
        (tokens.access_token, tokens.refresh_token)
    };
    // Save the new refresh_token immediately so a failure in the Xbox/XSTS/MC
    // chain does not lose the rotated token.
    let mut saved_account = account;
    saved_account.refresh_token = refresh_token.clone();
    update_account(uuid, &saved_account).await?;
    // This save is the load-bearing one, and it was the only step of the refresh
    // with no line of its own.
    log::debug!("The rotated refresh token for {uuid} is stored");
    let refreshed_account = access_token_auth_flow(&access_token, &refresh_token)
        .await
        .inspect_err(|error| {
            log::error!("The Xbox / XSTS / Minecraft chain failed for {uuid}: {error}");
        })?;
    update_account(uuid, &refreshed_account).await?;
    log::info!("Refreshed the account {uuid}");
    Ok(refreshed_account)
}

/// Makes `cape_id` the account's active cape on Mojang's side, then re-reads
/// and stores the profile so its `capes[].state` reflects the change.
///
/// The account is refreshed first when its token is close to expiry, so the
/// call does not ride an expired access token. Only a Microsoft account has
/// capes: a Yggdrasil profile's textures are its server's to change, not this
/// endpoint's, and the UI is expected to offer this for Microsoft accounts only.
pub async fn set_active_cape(uuid: Uuid, cape_id: &str) -> Result<MicrosoftAccount> {
    let account = refresh_account(uuid, false).await?;
    minecraft_profile_step::set_active_cape(&account.minecraft_access_token, cape_id).await?;
    reload_profile(&account).await
}

/// Clears the account's active cape on Mojang's side and stores the reloaded
/// profile. The account keeps owning every cape.
pub async fn clear_active_cape(uuid: Uuid) -> Result<MicrosoftAccount> {
    let account = refresh_account(uuid, false).await?;
    minecraft_profile_step::clear_active_cape(&account.minecraft_access_token).await?;
    reload_profile(&account).await
}

/// Re-reads the game profile and writes it back, so a cape change's new `state`
/// values reach the account file (and, later, the UI that reads it).
async fn reload_profile(account: &MicrosoftAccount) -> Result<MicrosoftAccount> {
    let response =
        minecraft_profile_step::get_game_profile(&account.minecraft_access_token).await?;
    let mut updated = account.clone();
    updated.profile = account_profile_step::generate_account_profile(response).await?;
    update_account(updated.profile.uuid, &updated).await?;
    Ok(updated)
}

async fn save_accounts(accounts: &[MicrosoftAccount]) -> Result<()> {
    let accounts_list_file = LOCATIONS.launcher.accounts.join("microsoft.json");
    let serialized_accounts_list = serde_json::to_string_pretty(accounts)?;
    tokio::fs::create_dir_all(&LOCATIONS.launcher.accounts).await?;
    tokio::fs::write(accounts_list_file, serialized_accounts_list).await?;
    Ok(())
}

pub async fn access_token_auth_flow(
    access_token: &str,
    refresh_token: &str,
) -> Result<MicrosoftAccount> {
    access_token_auth_flow_with_reporter(access_token, refresh_token, None).await
}

/// Like [`access_token_auth_flow`], but reports each stage of the chain
/// through the given [`LoginReporter`].
pub(crate) async fn access_token_auth_flow_with_reporter(
    access_token: &str,
    refresh_token: &str,
    reporter: Option<&LoginReporter>,
) -> Result<MicrosoftAccount> {
    fn report(reporter: Option<&LoginReporter>, event: LoginEvent) {
        if let Some(reporter) = reporter {
            reporter.report(event);
        }
    }

    info!("Starting access token auth flow");
    report(reporter, LoginEvent::XboxAuthenticate);
    let xbox_auth_response = xbox_auth_step::xbox_authenticate(access_token).await?;
    info!("Successfully login Xbox");

    report(reporter, LoginEvent::XstsAuthenticate);
    let xsts_token = xsts_auth_step::xsts_authenticate(&xbox_auth_response.xbl_token).await?;
    info!("Successfully verify XSTS");

    report(reporter, LoginEvent::MinecraftAuthenticate);
    let (minecraft_access_token, expires_in_secs) =
        minecraft_auth_step::minecraft_authenticate(&xbox_auth_response.xbl_uhs, &xsts_token)
            .await?;
    info!("Successfully get Minecraft access token");

    report(reporter, LoginEvent::GetProfile);
    let minecraft_profile_response =
        minecraft_profile_step::get_game_profile(&minecraft_access_token).await?;
    info!("Successfully get game profile");

    Ok(MicrosoftAccount {
        refresh_token: refresh_token.to_string(),
        minecraft_access_token,
        expires_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Incorrect System Time")
            .as_secs()
            + expires_in_secs,
        profile: account_profile_step::generate_account_profile(minecraft_profile_response).await?,
    })
}
