use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

#[derive(Clone, Serialize, Deserialize)]
struct XSTSAuthProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct XSTSAuthBody {
    #[serde(rename = "Properties")]
    properties: XSTSAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

impl XSTSAuthBody {
    fn new(xbl_token: &str) -> Self {
        Self {
            properties: XSTSAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_token.to_string()],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        }
    }
}

pub(super) async fn xsts_authenticate(xbl_token: &str) -> Result<String> {
    let response: Value = HTTP_CLIENT
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(serde_json::to_string(&XSTSAuthBody::new(xbl_token))?)
        .send()
        .await?
        .json()
        .await?;
    Ok(response["Token"]
        .as_str()
        .ok_or(Error::MicrosoftResponseMissingKey("Token".to_string()))?
        .to_string())
}
