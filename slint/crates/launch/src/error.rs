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
    #[error("Another instance is launching")]
    AnothorInstanceLaunching,
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

    #[error("Invalid Profile")]
    InvalidProfile,

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

    #[error("No suitable Java runtime found")]
    NoSuitableJavaRuntime,

    // The account crate's error is not `Serialize` in the Slint workspace (the
    // mirror drops its IPC-boundary derives), and the Slint app never serializes
    // a launch error, so this variant is left out of the serialized form.
    #[serde(skip)]
    #[error(transparent)]
    AccountError(#[from] slint_account::Error),

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tokio::task::JoinError,
    ),

    #[error("{0}")]
    ChecksumMissmatch(String),

    #[error("Unhandled Error")]
    Other,
}

impl From<slint_version::Error> for Error {
    fn from(value: slint_version::Error) -> Self {
        match value {
            slint_version::Error::Io(error) => Self::Io(error),
            slint_version::Error::JsonParse(error) => Self::VersionJsonParse(error),
            slint_version::Error::InvalidVersionJson => Self::InvalidVersionJson("".to_string()),
            slint_version::Error::InvalidMinecraftVersion => Self::InvalidMinecraftVersion,
        }
    }
}

impl From<slint_instance::Error> for Error {
    fn from(value: slint_instance::Error) -> Self {
        match value {
            slint_instance::Error::Io(error) => Self::Io(error),
            slint_instance::Error::TomlSer(error) => Self::InvalidInstance(error.to_string()),
            slint_instance::Error::TomlDe(error) => Self::InvalidInstance(error.to_string()),
            slint_instance::Error::InvalidInstanceConfig => {
                Self::InvalidInstance("Invalid instance config".to_string())
            }
        }
    }
}

impl From<slint_install::Error> for Error {
    fn from(value: slint_install::Error) -> Self {
        match value {
            slint_install::Error::Io(error) => Self::Io(error),
            slint_install::Error::Network(error) => Self::Network(error),
            slint_install::Error::InstanceBroken => Self::InvalidInstance("".to_string()),
            slint_install::Error::JsonParse(error) => Self::VersionJsonParse(error),
            slint_install::Error::InvalidVersionJson(error) => Self::InvalidVersionJson(error),
            _ => Self::Other,
        }
    }
}

impl From<slint_download::Error> for Error {
    fn from(value: slint_download::Error) -> Self {
        match value {
            slint_download::Error::Io(e) => Self::Io(e),
            slint_download::Error::ChecksumMissmatch(e) => Self::ChecksumMissmatch(e),
            slint_download::Error::Network(e) => Self::Network(e),
            slint_download::Error::ChunkLengthMismatch => Self::ChunkLengthMismatch,
            slint_download::Error::UrlParse(_) => Self::Other,
            slint_download::Error::Aborted(error) => Self::Aborted(error),
        }
    }
}

impl From<slint_java_runtime::Error> for Error {
    fn from(value: slint_java_runtime::Error) -> Self {
        match value {
            slint_java_runtime::Error::Io(error) => Self::Io(error),
            slint_java_runtime::Error::NoSuitableJavaRuntime => Self::NoSuitableJavaRuntime,
            _ => Self::Other,
        }
    }
}
