// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::result;

use thiserror::Error;
use uuid::Uuid;

use shared::UrlExtError;

pub type Result<T> = result::Result<T, Error>;

/// Every failure the account flows can end in.
///
/// These messages are user-visible: callers read them through
/// [`ToString::to_string`], so they are worded to be shown as-is.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Another login task is already running")]
    LoginInProgress,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error(transparent)]
    InvalidBaseUrl(#[from] UrlExtError),

    #[error(transparent)]
    JsonParse(#[from] serde_json::error::Error),

    #[error(transparent)]
    ToStr(#[from] reqwest::header::ToStrError),

    #[error(transparent)]
    Network(#[from] reqwest::Error),

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
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("The device code has expired, please try again")]
    DeviceCodeExpired,

    #[error("The authorization was declined")]
    AuthorizationDeclined,

    #[error("Invalid device code, please try again")]
    BadVerificationCode,

    #[error("HTTP request failed with status {status}: {body}")]
    HttpResponse { status: u16, body: String },

    #[error(transparent)]
    Aborted(#[from] tokio::task::JoinError),
}
