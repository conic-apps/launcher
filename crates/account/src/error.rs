// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

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
    #[error("Another login task is already running")]
    LoginInProgress,

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
    InvalidBaseUrl(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        shared::UrlExtError,
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

    #[error("{0}")]
    MicrosoftResponseMissingKey(String),

    #[error("Unable to parse yggdrasil server api location, please ask your server for help")]
    InvalidALIResponse,

    #[error("Unable to parse texture")]
    YggdrasilTextureParseError,

    #[error(transparent)]
    Base64DecodeError(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        base64::DecodeError,
    ),

    #[error("The device code has expired, please try again")]
    DeviceCodeExpired,

    #[error("The authorization was declined")]
    AuthorizationDeclined,

    #[error("Invalid device code, please try again")]
    BadVerificationCode,

    #[error("HTTP request failed with status {status}: {body}")]
    HttpResponse { status: u16, body: String },

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tokio::task::JoinError,
    ),
}
