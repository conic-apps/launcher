use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

pub async fn get_access_token(code: &str) -> Result<(String, String)> {
    let response: Value = HTTP_CLIENT
        .post("https://login.live.com/oauth20_token.srf")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=authorization_code"
                + "&code="
                + code
                + "&redirect_uri=https://login.live.com/oauth20_desktop.srf"
                + "&scope=service::user.auth.xboxlive.com::MBI_SSL",
        )
        .send()
        .await?
        .json()
        .await?;
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
    Ok((access_token, refresh_token))
}

pub(super) async fn get_access_token_from_refresh_token(
    refresh_token: &str,
) -> Result<(String, String)> {
    let response: Value = HTTP_CLIENT
        .post("https://login.live.com/oauth20_token.srf")
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(
            "client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc".to_string()
                + "&grant_type=refresh_token"
                + "&refresh_token="
                + refresh_token
                + "&redirect_uri=https://login.live.com/oauth20_desktop.srf"
                + "&scope=service::user.auth.xboxlive.com::MBI_SSL",
        )
        .send()
        .await?
        .json()
        .await?;
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
    Ok((access_token, refresh_token))
}
