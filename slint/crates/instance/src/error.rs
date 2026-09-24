// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

/// Errors produced while reading or writing game instances.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("failed to serialize the instance config: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("failed to deserialize the instance config: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("invalid instance config")]
    InvalidInstanceConfig,
}

pub type Result<T> = std::result::Result<T, Error>;
