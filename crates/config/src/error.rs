// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::io;

/// Errors produced while loading, saving or resetting the configuration.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read or write the configuration: {0}")]
    Io(#[from] io::Error),

    #[error("failed to serialize the configuration: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("failed to deserialize the configuration: {0}")]
    TomlDe(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
