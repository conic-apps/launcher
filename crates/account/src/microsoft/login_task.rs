// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::{info, warn};
use shared::Sink;

use crate::{
    error::*,
    microsoft::{self, MicrosoftAccount, access_token_auth_flow_with_reporter, device_code},
};

/// How many polls in a row may fail outright before the device code flow gives
/// up. A pending or backed-off poll is an answer and not counted here — only a
/// request that never reached an OAuth answer.
const MAX_FAILED_POLLS: u32 = 3;

/// Progress events reported while a Microsoft login task is running.
///
/// The UI maps most variants to a progress string; `WaitingForAuthorization`
/// instead carries the device-code screen's data. No `serde` derives are
/// needed: the events cross no process boundary.
#[derive(Debug, Clone, PartialEq)]
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
///
/// The flow runs off the UI thread, so the closure the app supplies forwards
/// each event with `upgrade_in_event_loop`. The mechanism is [`shared::Sink`],
/// the same one every other crate's progress port uses; only the vocabulary
/// ([`LoginEvent`]) is this crate's.
#[derive(Clone)]
pub struct LoginReporter {
    on_event: Sink<LoginEvent>,
}

impl LoginReporter {
    /// A reporter that ignores every event, for a caller that has no UI to
    /// report to.
    pub fn silent() -> Self {
        Self {
            on_event: shared::silent(),
        }
    }

    pub fn new(on_event: impl Fn(LoginEvent) + Send + Sync + 'static) -> Self {
        Self {
            on_event: Arc::new(on_event),
        }
    }

    /// Reports one progress event. A reporter whose sink has gone away drops
    /// it: the window may already be closed while the task winds down.
    pub fn report(&self, event: LoginEvent) {
        info!("Microsoft login progress: {event:?}");
        (self.on_event)(event);
    }
}

/// Logs in using an authorization code obtained from the browser flow.
///
/// `redirect_uri` is the URI the code was issued against, which the token
/// request has to repeat exactly — see [`microsoft::redeem_access_token`].
pub(crate) async fn login_with_auth_code(
    code: &str,
    redirect_uri: &str,
    reporter: &LoginReporter,
) -> Result<MicrosoftAccount> {
    reporter.report(LoginEvent::RedeemAccessToken);
    let (access_token, refresh_token) = {
        let tokens = microsoft::redeem_access_token(code, redirect_uri).await?;
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
    let mut interval = Duration::from_secs(response.interval.max(1));
    let mut failed_polls = 0;
    let tokens = loop {
        tokio::time::sleep(interval).await;
        if Instant::now() >= deadline {
            return Err(Error::DeviceCodeExpired);
        }
        let poll_result = match device_code::poll_device_code(&response.device_code).await {
            Ok(poll_result) => {
                failed_polls = 0;
                poll_result
            }
            // The user is given minutes to finish in the browser, and the flow
            // asks for a new code (a new code, a new wait) when this one
            // expires: a single unreachable request should not throw away a
            // code they are in the middle of authorizing. A few in a row
            // means the network is really gone, so the error is reported.
            Err(error) => {
                failed_polls += 1;
                if failed_polls < MAX_FAILED_POLLS {
                    warn!("device code poll failed ({failed_polls}/{MAX_FAILED_POLLS}): {error}");
                    continue;
                }
                return Err(error);
            }
        };
        match poll_result.status.as_str() {
            "success" => break poll_result,
            "authorization_pending" => {}
            // RFC 8628: back off by 5s, for this and every later poll.
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
