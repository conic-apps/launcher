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
    #[error("Another instance is launching")]
    AlreadyInLaunching,
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),
    #[error(transparent)]
    VersionJsonParse(#[serde_as(as = "serde_with::DisplayFromStr")] serde_json::Error),

    #[error("Bad Version JSON, missing: {0}")]
    InvalidVersionJson(String),

    #[error("Invalid Minecraft version")]
    InvalidMinecraftVersion,

    #[error("Instance broken: {0}")]
    InvalidInstance(String),

    #[error(transparent)]
    DecompressionFailed(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        zip::result::ZipError,
    ),

    #[error(transparent)]
    Network(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        reqwest::Error,
    ),

    #[error("Chunk length mismatch")]
    ChunkLengthMismatch,

    #[error("Unabled to take Minecraft stdout")]
    TakeMinecraftStdoutFailed,

    #[error(transparent)]
    AccountError(#[from] account::Error),

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        futures::future::Aborted,
    ),

    #[error("{0}")]
    Sha1Missmatch(String),

    #[error("Unhandled Error")]
    Other,
}

impl From<version::Error> for Error {
    fn from(value: version::Error) -> Self {
        match value {
            version::Error::Io(error) => Self::Io(error),
            version::Error::JsonParse(error) => Self::VersionJsonParse(error),
            version::Error::InvalidVersionJson => Self::InvalidVersionJson("".to_string()),
            version::Error::InvalidMinecraftVersion => Self::InvalidMinecraftVersion,
        }
    }
}

impl From<instance::Error> for Error {
    fn from(value: instance::Error) -> Self {
        match value {
            instance::Error::Io(error) => Self::Io(error),
            instance::Error::TomlSerialize(error) => Self::InvalidInstance(error.to_string()),
            instance::Error::InvalidInstanceConfig => {
                Self::InvalidInstance("Invalid instance config".to_string())
            }
        }
    }
}

impl From<install::Error> for Error {
    fn from(value: install::Error) -> Self {
        match value {
            install::Error::Io(error) => Self::Io(error),
            install::Error::Network(error) => Self::Network(error),
            install::Error::InstanceBroken => Self::InvalidInstance("".to_string()),
            install::Error::JsonParse(error) => Self::VersionJsonParse(error),
            install::Error::InvalidVersionJson(error) => Self::InvalidVersionJson(error),
            _ => Self::Other,
        }
    }
}

impl From<download::Error> for Error {
    fn from(value: download::Error) -> Self {
        match value {
            download::Error::Io(e) => Self::Io(e),
            download::Error::Sha1Missmatch(e) => Self::Sha1Missmatch(e),
            download::Error::Network(e) => Self::Network(e),
            download::Error::ChunkLengthMismatch => Self::ChunkLengthMismatch,
            download::Error::UrlParse(_) => Self::Other,
        }
    }
}
