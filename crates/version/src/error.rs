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
    JsonParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        serde_json::error::Error,
    ),

    #[error("Bad Version JSON")]
    InvalidVersionJson,

    #[error("Invalid Minecraft version")]
    InvalidMinecraftVersion,
}
