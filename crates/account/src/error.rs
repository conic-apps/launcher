use std::result;

use serde::Serialize;
use serde_with::serde_as;
use thiserror::Error;
use uuid::Uuid;

pub type Result<T> = result::Result<T, Error>;

#[serde_as]
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum Error {
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),

    #[error(transparent)]
    UrlParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        url::ParseError,
    ),

    #[error(transparent)]
    JsonParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        serde_json::error::Error,
    ),

    #[error(transparent)]
    ToStr(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::header::ToStrError,
    ),

    #[error(transparent)]
    Network(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::Error,
    ),

    #[error("Account not found: {0}")]
    AccountNotfound(Uuid),

    #[error("This profile is no longer available")]
    ProfileUnavailable,

    #[error("Cannot check ownership")]
    OwnershipCheckFailed,

    #[error("{0}")]
    MicrosoftResponseMissingKey(String),
}
