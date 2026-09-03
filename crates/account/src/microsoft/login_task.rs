// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{Duration, Instant};

use log::info;
use serde::Serialize;
use tauri::ipc::Channel;

use crate::{
    error::*,
    microsoft::{self, MicrosoftAccount, access_token_auth_flow_with_reporter, device_code},
};

/// Progress events reported while a Microsoft login task is running.
///
/// Serialized with the same `job`/`progress` tagging as install/launch events.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "job", content = "progress")]
pub enum LoginEvent {
    Prepare,
    RequestDeviceCode,
    WaitingForAuthorization {
        user_code: String,
        verification_uri: String,
        expires_in: u64,
        interval: u64,
    },
    RedeemAccessToken,
    XboxAuthenticate,
    XstsAuthenticate,
    MinecraftAuthenticate,
    GetProfile,
    SaveAccount,
}

/// Clonable handle through which the login flow reports [`LoginEvent`]s.
#[derive(Clone)]
pub struct LoginReporter {
    channel: Channel<LoginEvent>,
}

impl LoginReporter {
    pub fn new(channel: Channel<LoginEvent>) -> Self {
        Self { channel }
    }

    /// Sends one progress event to the frontend. A failed send is ignored:
    /// the webview may already be gone while the task winds down.
    pub fn report(&self, event: LoginEvent) {
        info!("Microsoft login progress: {event:?}");
        let _ = self.channel.send(event);
    }
}

/// Logs in using an authorization code obtained from the browser flow.
pub(crate) async fn login_with_auth_code(
    code: &str,
    reporter: &LoginReporter,
) -> Result<MicrosoftAccount> {
    reporter.report(LoginEvent::RedeemAccessToken);
    let (access_token, refresh_token) = {
        let tokens = microsoft::redeem_access_token(code).await?;
        (tokens.access_token, tokens.refresh_token)
    };
    finish_login(access_token, refresh_token, reporter).await
}

/// Runs the device-code login flow: requests a device code, waits for the user
/// to authorize it on any device, then completes the authentication chain.
pub(crate) async fn login_with_device_code(reporter: &LoginReporter) -> Result<MicrosoftAccount> {
    reporter.report(LoginEvent::RequestDeviceCode);
    let response = device_code::request_device_code().await?;
    reporter.report(LoginEvent::WaitingForAuthorization {
        user_code: response.user_code.clone(),
        verification_uri: response.verification_uri.clone(),
        expires_in: response.expires_in,
        interval: response.interval,
    });

    let deadline = Instant::now() + Duration::from_secs(response.expires_in);
    let mut interval = Duration::from_secs(response.interval);
    let tokens = loop {
        tokio::time::sleep(interval).await;
        if Instant::now() >= deadline {
            return Err(Error::DeviceCodeExpired);
        }
        let poll_result = device_code::poll_device_code(&response.device_code).await?;
        match poll_result.status.as_str() {
            "success" => break poll_result,
            "authorization_pending" => {}
            "slow_down" => interval += Duration::from_secs(5),
            "authorization_declined" => return Err(Error::AuthorizationDeclined),
            "bad_verification_code" => return Err(Error::BadVerificationCode),
            "expired_token" => return Err(Error::DeviceCodeExpired),
            unexpected => {
                return Err(Error::MicrosoftResponseMissingKey(unexpected.to_string()));
            }
        }
    };

    let access_token = tokens
        .access_token
        .ok_or_else(|| Error::MicrosoftResponseMissingKey("access_token".to_string()))?;
    let refresh_token = tokens
        .refresh_token
        .ok_or_else(|| Error::MicrosoftResponseMissingKey("refresh_token".to_string()))?;
    finish_login(access_token, refresh_token, reporter).await
}

/// Completes the shared part of both flows: runs the Xbox/XSTS/Minecraft auth
/// chain and persists the resulting account.
async fn finish_login(
    access_token: String,
    refresh_token: String,
    reporter: &LoginReporter,
) -> Result<MicrosoftAccount> {
    let account =
        access_token_auth_flow_with_reporter(&access_token, &refresh_token, Some(reporter)).await?;
    reporter.report(LoginEvent::SaveAccount);
    microsoft::add_account(account.clone()).await?;
    Ok(account)
}
