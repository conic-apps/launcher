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

async fn check_response(response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(Error::HttpResponse {
            status: status.as_u16(),
            body,
        });
    }
    serde_json::from_str(&body).map_err(Into::into)
}

/// Trades an authorization code for a token pair.
///
/// `redirect_uri` has to be the URI the code was issued against, byte for
/// byte — that is the whole of RFC 6749 §4.1.3, and it is why the Slint app
/// cannot keep the original's hard-coded `conic-launcher://…`: its listener
/// binds an OS-chosen port, so the URI only exists once the flow is under way.
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
    let response = check_response(response).await?;
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
/// `reqwest`'s `form()` would do this, and would rebuild the body out of a
/// `serde` map; the body is hand-built here because that is how the original
/// crate writes it, and the two requests it makes here have to keep matching
/// it line for line. The one character that matters in a `redirect_uri` is the
/// `:` and the two `/` — which is exactly what the original's literal
/// `conic-launcher%3A%2F%2Foauth2%2Fmicrosoft%2Fcallback` encoded.
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
        // No `redirect_uri`, where the original sends a hard-coded
        // `conic-launcher://…`. RFC 6749 §6 makes it conditional — it is only
        // required when the original authorization request carried one, and
        // every refresh this crate issues follows a device-code authorization,
        // which never did. It is not among the parameters Microsoft's token
        // endpoint documents for this grant either, so what the original sends
        // was never read. Carrying a URI that this app does not use would be
        // worse than not sending one.
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=refresh_token"
                + "&refresh_token="
                + refresh_token
                + "&scope=XboxLive.signin%20offline_access",
        )
        .send()
        .await?;
    let response = check_response(response).await?;
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
