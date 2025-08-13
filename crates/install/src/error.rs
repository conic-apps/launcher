// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::result;

use serde::Serialize;
use serde_with::serde_as;
use thiserror::Error;

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
    Network(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::Error,
    ),
    #[error("Bad instance.toml file")]
    InstanceBroken,
    #[error("Could not understand forge version")]
    InvalidForgeVersion,
    #[error("Failed to run forge installer")]
    ForgeInstallerFailed,
    #[error("Failed to run neoforged installer")]
    NeoforgedInstallerFailed,
    #[error("{0} {1}")]
    HttpResponseNotSuccess(u16, String),
    #[error("Invalid version.json, missing {0}")]
    InvalidVersionJson(String),
    #[error("Version metadata not found in version manifest")]
    VersionMetadataNotfound,
    #[error(transparent)]
    JsonParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        serde_json::Error,
    ),
    #[error(transparent)]
    ResolveVersionJsonFailed(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        version::error::Error,
    ),
    #[error("{0}")]
    Sha1Missmatch(String),

    #[error(transparent)]
    UrlParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        url::ParseError,
    ),

    #[error("No supported java runtime")]
    NoSupportedJavaRuntime,
}

impl From<download::error::Error> for Error {
    fn from(value: download::error::Error) -> Self {
        match value {
            download::error::Error::Io(error) => Self::Io(error),
            download::error::Error::Sha1Missmatch(error) => Self::Sha1Missmatch(error),
            download::error::Error::Network(error) => Self::Network(error),
            download::error::Error::HttpResponseNotSuccess(code, message) => {
                Self::HttpResponseNotSuccess(code, message)
            }
            download::error::Error::UrlParse(error) => Self::UrlParse(error),
        }
    }
}
