// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde_json::Value;

use crate::error::*;
use shared::HTTP_CLIENT;

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// Reads the token endpoint's answer.
///
/// The body is the only place the endpoint explains itself — a rejected grant
/// arrives as a `400` carrying `error` and `error_description` — and it was read
/// and then dropped without a word. `invalid_grant` in particular means the user
/// revoked the app or the refresh token expired, which is the one thing a user
/// asking "why did I have to sign in again" needs to hear.
async fn check_response(response: reqwest::Response, what: &str) -> Result<Value> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        // Truncated: a proxy in front of the endpoint can return a whole HTML page.
        let shown: String = body.chars().take(512).collect();
        log::warn!("{what} was refused with {status}: {shown}");
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    serde_json::from_str(&body).map_err(Into::into)
}

/// Trades an authorization code for a token pair.
///
/// `redirect_uri` has to be the URI the code was issued against, byte for byte
/// — RFC 6749 §4.1.3. It is a parameter rather than a literal because the
/// loopback listener binds an OS-chosen port, so the URI only exists once the
/// flow is under way.
pub async fn redeem_access_token(code: &str, redirect_uri: &str) -> Result<TokenPair> {
    let response = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=authorization_code"
                + "&code="
                + code
                + "&redirect_uri="
                + &urlencoding(redirect_uri)
                + "&scope=XboxLive.signin%20offline_access",
        )
        .send()
        .await?;
    // The code and the refresh token are secrets, so neither is logged — only
    // what was attempted and whether it worked.
    let response = check_response(response, "redeeming the authorization code").await?;
    let access_token = response["access_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "access_token".to_string(),
        ))?
        .to_string();
    let refresh_token = response["refresh_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "refresh_token".to_string(),
        ))?
        .to_string();
    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}

/// Percent-encodes a form value.
///
/// The request bodies are hand-built form strings rather than `reqwest`'s
/// `form()`, which would rebuild them from a `serde` map. The characters that
/// matter in a `redirect_uri` are its `:` and the two `/`, which this encodes
/// as `%3A%2F%2F`.
fn urlencoding(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte))
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

pub(super) async fn get_access_token_from_refresh_token(refresh_token: &str) -> Result<TokenPair> {
    let response = HTTP_CLIENT
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        // No `redirect_uri`: RFC 6749 §6 makes it conditional — required only
        // when the authorization request carried one. An account keeps its
        // refresh token but not the URI the code was issued against, so a
        // refresh has none to send.
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=refresh_token"
                + "&refresh_token="
                + refresh_token
                + "&scope=XboxLive.signin%20offline_access",
        )
        .send()
        .await?;
    // This runs before every launch that needs a fresh token, so it is one of the
    // most frequent requests the launcher makes and it used to leave no trace.
    let response =
        check_response(response, "exchanging the refresh token for an access token").await?;
    let access_token = response["access_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "access_token".to_string(),
        ))?
        .to_string();
    let refresh_token = response["refresh_token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey(
            "refresh_token".to_string(),
        ))?
        .to_string();
    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}
