// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::result;

use thiserror::Error;
use uuid::Uuid;

use crate::shared::UrlExtError;

pub type Result<T> = result::Result<T, Error>;

/// Every failure the account flows can end in, variant for variant as
/// `crates/account/src/error.rs` declares it.
///
/// The original derives `Serialize` (through `serde_with`) so a command can
/// hand the error to the webview, where `crates/account/index.ts` models it as
/// `{ kind, message }`. There is no IPC boundary here, so the derives are gone
/// and callers read [`std::error::Error::to_string`] — the same text the
/// frontend showed, because the messages are the originals'.
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
