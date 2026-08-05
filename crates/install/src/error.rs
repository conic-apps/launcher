// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
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
    #[error("Another instance is installing")]
    AlreadyInstalling,
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),
    #[error("Invalid instance config")]
    InvalidInstanceConfig,
    #[error(transparent)]
    Network(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::Error,
    ),
    #[error("Bad instance.toml file")]
    InstanceBroken,
    #[error(transparent)]
    Zip(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        zip::result::ZipError,
    ),
    #[error("No available forge version")]
    NoAvailableForgeVersion,
    #[error("Could not understand forge version")]
    InvalidForgeVersion,
    #[error("Failed to run forge installer")]
    ForgeInstallerFailed,
    #[error("Failed to run neoforge installer")]
    NeoforgeInstallerFailed,
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
        version::Error,
    ),
    #[error("{0}")]
    ChecksumMissmatch(String),

    #[error(transparent)]
    UrlParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        url::ParseError,
    ),

    #[error("No supported java runtime")]
    NoSupportedJavaRuntime,

    #[error("Invalid authlib version response")]
    InvalidAuthlibResponse,

    #[error("Chunk length mismatch")]
    ChunkLengthMismatch,

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tokio::task::JoinError,
    ),
}

impl From<download::Error> for Error {
    fn from(value: download::Error) -> Self {
        match value {
            download::Error::Io(error) => Self::Io(error),
            download::Error::ChecksumMissmatch(error) => Self::ChecksumMissmatch(error),
            download::Error::Network(error) => Self::Network(error),
            download::Error::UrlParse(error) => Self::UrlParse(error),
            download::Error::ChunkLengthMismatch => Self::ChunkLengthMismatch,
            download::Error::Aborted(error) => Self::Aborted(error),
        }
    }
}
