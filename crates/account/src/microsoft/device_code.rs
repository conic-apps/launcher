// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::*;
use shared::HTTP_CLIENT;

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

pub async fn request_device_code() -> Result<DeviceCodeResponse> {
    let response = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc&scope=XboxLive.signin offline_access"
                .to_string(),
        )
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        let shown: String = body.chars().take(512).collect();
        log::warn!("the device code request was refused with {status}: {shown}");
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    let code: DeviceCodeResponse = serde_json::from_str(&body)?;
    // The interval and the expiry drive the countdown the dialog shows, and a
    // wrong value here is the whole explanation for a code that "expired" while
    // the user was still typing it.
    log::info!(
        "the device code was issued: verify at {}, expiring in {} seconds, polling every \
         {}",
        code.verification_uri,
        code.expires_in,
        code.interval
    );
    Ok(code)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceCodePollResult {
    pub status: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

/// Polls the token endpoint once for a device code.
///
/// Every state of the flow other than success — `authorization_pending` and
/// `slow_down` while the user is still in the browser, but also
/// `authorization_declined` and `expired_token` — comes back as
/// `400 Bad Request` with an OAuth `error` in the body. Those are answers, not
/// failed requests, so the body is read before the status is judged and the
/// `error` is handed back as the [`DeviceCodePollResult::status`] for the
/// caller to keep polling on. Only a response without a readable OAuth error
/// (a proxy's HTML error page, an empty 5xx) is a real [`Error::HttpResponse`].
pub async fn poll_device_code(device_code: &str) -> Result<DeviceCodePollResult> {
    let raw_response = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "grant_type=urn:ietf:params:oauth:grant-type:device_code".to_string()
                + "&client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc"
                + "&device_code="
                + device_code,
        )
        .send()
        .await?;
    let status = raw_response.status();
    let body = raw_response.text().await?;
    poll_result_from_response(status, body)
}

/// The body half of [`poll_device_code`], split out so the states the endpoint
/// answers with can be tested without a network round trip.
fn poll_result_from_response(status: StatusCode, body: String) -> Result<DeviceCodePollResult> {
    let response: Value = match serde_json::from_str(&body) {
        Ok(response) => response,
        Err(_) if !status.is_success() => {
            // No OAuth error in an answer that was not a success: a proxy's HTML
            // page, or a 5xx with an empty body. Nothing here is an OAuth state,
            // so it is a failed request rather than "keep polling".
            let shown: String = body.chars().take(512).collect();
            log::warn!("the device code poll answered {status} with no OAuth error: {shown}");
            return Err(Error::HttpResponse {
                status: status.as_u16(),
                body,
            });
        }
        Err(error) => return Err(error.into()),
    };
    if let Some(error) = response["error"].as_str() {
        // Every state of the flow other than success arrives as a `400` with an
        // OAuth error in it, and the caller decides which of them keep it going.
        // `authorization_pending` is the ordinary one and would be noise per
        // poll; the rest change what happens next, so they are recorded.
        match error {
            "authorization_pending" => {}
            state => log::debug!("the device code poll answered {state}"),
        }
        return Ok(DeviceCodePollResult {
            status: error.to_string(),
            access_token: None,
            refresh_token: None,
            expires_in: None,
        });
    }
    if !status.is_success() {
        log::warn!("the device code poll answered {status} with no OAuth error");
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    log::debug!("the device code poll succeeded");

    Ok(DeviceCodePollResult {
        status: "success".to_string(),
        access_token: response["access_token"].as_str().map(|s| s.to_string()),
        refresh_token: response["refresh_token"].as_str().map(|s| s.to_string()),
        expires_in: response["expires_in"].as_u64(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poll(status: StatusCode, body: &str) -> Result<DeviceCodePollResult> {
        poll_result_from_response(status, body.to_string())
    }

    /// The states that keep the flow going all arrive as `400`s. Reading one as
    /// a failed request is what ended the login on the first poll, before the
    /// user had opened the browser at all.
    #[test]
    fn oauth_errors_are_statuses_even_on_a_bad_request() {
        for error in [
            "authorization_pending",
            "slow_down",
            "authorization_declined",
            "bad_verification_code",
            "expired_token",
        ] {
            let result = poll(
                StatusCode::BAD_REQUEST,
                &format!(r#"{{"error":"{error}","error_description":"..."}}"#),
            )
            .unwrap_or_else(|error| panic!("{error} was read as a failed request: {error:?}"));
            assert_eq!(result.status, error);
            assert!(result.access_token.is_none());
        }
    }

    #[test]
    fn a_successful_poll_carries_the_tokens() {
        let result = poll(
            StatusCode::OK,
            r#"{"access_token":"access","refresh_token":"refresh","expires_in":3599}"#,
        )
        .expect("a token response is a success");
        assert_eq!(result.status, "success");
        assert_eq!(result.access_token.as_deref(), Some("access"));
        assert_eq!(result.refresh_token.as_deref(), Some("refresh"));
        assert_eq!(result.expires_in, Some(3599));
    }

    /// A body that is not an OAuth answer at all is still a failed request.
    #[test]
    fn a_body_without_an_oauth_error_fails_the_request() {
        for (status, body) in [
            (StatusCode::BAD_GATEWAY, "<html>502</html>"),
            (StatusCode::BAD_REQUEST, "not json"),
        ] {
            assert!(matches!(
                poll(status, body),
                Err(Error::HttpResponse { status: code, .. }) if code == status.as_u16()
            ));
        }
    }
}
