use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

pub(super) struct XboxAuth {
    pub(super) xbl_token: String,
    pub(super) xbl_uhs: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct XboxAuthProperties {
    #[serde(rename = "AuthMethod")]
    pub(super) auth_method: String,
    #[serde(rename = "SiteName")]
    pub(super) site_name: String,
    #[serde(rename = "RpsTicket")]
    pub(super) rps_ticket: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct XboxAuthBody {
    #[serde(rename = "Properties")]
    pub(super) properties: XboxAuthProperties,
    #[serde(rename = "RelyingParty")]
    pub(super) relying_party: String,
    #[serde(rename = "TokenType")]
    pub(super) token_type: String,
}

impl XboxAuthBody {
    fn new(access_token: &str) -> Self {
        Self {
            properties: XboxAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: format!("d={access_token}"),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        }
    }
}

pub(crate) async fn xbox_authenticate(access_token: &str) -> Result<XboxAuth> {
    let response: Value = HTTP_CLIENT
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(serde_json::to_string(&XboxAuthBody::new(access_token))?)
        .send()
        .await?
        .json()
        .await?;
    Ok(XboxAuth {
        xbl_token: response["Token"]
            .as_str()
            .ok_or(Error::MicrosoftResponseMissingKey("xbl Token".to_string()))?
            .to_string(),
        xbl_uhs: response["DisplayClaims"]["xui"][0]["uhs"]
            .as_str()
            .ok_or(Error::MicrosoftResponseMissingKey("xui_uhs".to_string()))?
            .to_string(),
    })
}
