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
    #[error("NBT path not found")]
    NBTPathNotFound,
    #[error("Is not compound")]
    IsNotCompound,
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),
    #[error(transparent)]
    NBTParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        fastnbt::error::Error,
    ),
    #[error(transparent)]
    JSONParse(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        serde_json::error::Error,
    ),
    #[error("Bad Pack")]
    BadPack,
    #[error(transparent)]
    Zip(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        zip::result::ZipError,
    ),
    #[error(transparent)]
    WorldMap(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        conic_worldmap::WorldError,
    ),
    #[error("World map render task failed: {0}")]
    WorldMapTask(String),
    #[error("World map PNG encoding failed: {0}")]
    WorldMapPng(String),
    #[error("Not a mod file")]
    NotAModFile,
    #[error("Failed to parse mod metadata: {0}")]
    ModParseFailed(String),
}
