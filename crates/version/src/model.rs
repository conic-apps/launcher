// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{collections::HashMap, str::FromStr};

use crate::error::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize)]
pub struct Arguments {
    pub game: Option<Vec<Value>>,
    pub jvm: Option<Vec<Value>>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    // pub sha1: String,
    pub size: u64,
    pub url: String,
    pub id: String,
    pub total_size: u64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Download {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
}

impl Default for JavaVersion {
    fn default() -> Self {
        Self {
            component: "jre-legacy".to_string(),
            major_version: 8,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Logging {
    pub file: LoggingFileDownload,
    pub argument: String,
    pub r#type: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LoggingFileDownload {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// The raw json format provided by Minecraft.
///
/// Use `parse` to parse a Minecraft version json, and see the detail info of the version.
///
/// With `ResolvedVersion`, you can use the resolved version to launch the game.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub time: Option<String>,
    pub r#type: Option<String>,
    pub release_time: Option<String>,
    pub inherits_from: Option<String>,
    pub minimum_launcher_version: Option<i32>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    pub main_class: Option<String>,
    pub libraries: Option<Vec<Value>>,
    pub jar: Option<String>,
    pub asset_index: Option<AssetIndex>,
    pub assets: Option<String>,
    pub downloads: Option<HashMap<String, Download>>,
    pub client: Option<String>,
    pub server: Option<String>,
    pub logging: Option<HashMap<String, Logging>>,
    pub java_version: Option<JavaVersion>,
    pub client_version: Option<String>,
}

impl FromStr for Version {
    type Err = crate::Error;
    fn from_str(raw: &str) -> std::result::Result<Version, crate::Error> {
        Ok(serde_json::from_str(raw)?)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AssetIndexObjectInfo {
    pub hash: String,
    pub size: u64,
}

pub type AssetIndexObject = HashMap<String, AssetIndexObjectInfo>;

#[derive(Clone, Deserialize, Serialize)]
pub struct LoggingFile {
    pub size: u64,
    pub url: String,
    pub id: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum LaunchArgument {
    String(String),
    Object(serde_json::map::Map<String, Value>),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Platform {
    pub name: String,
    pub version: Option<String>,
}

/// Minecraft Version
///
/// It used to compare the version of the game
#[derive(Clone, Serialize)]
pub enum MinecraftVersion {
    Release(u8, u8, Option<u8>),
    Snapshot(u8, u8, String),
    Unknown(String),
}

impl FromStr for MinecraftVersion {
    type Err = Error;
    fn from_str(raw: &str) -> std::result::Result<Self, Self::Err> {
        parse_version(raw)
    }
}

fn parse_version(raw: &str) -> Result<MinecraftVersion> {
    if raw.contains(".") {
        let split = raw.split(".").collect::<Vec<&str>>();
        Ok(MinecraftVersion::Release(
            #[allow(clippy::get_first)]
            split
                .get(0)
                .ok_or(Error::InvalidMinecraftVersion)?
                .parse()
                .map_err(|_| Error::InvalidMinecraftVersion)?,
            split
                .get(1)
                .ok_or(Error::InvalidMinecraftVersion)?
                .parse()
                .map_err(|_| Error::InvalidMinecraftVersion)?,
            match split.get(2) {
                Some(x) => Some(x.parse().map_err(|_| Error::InvalidMinecraftVersion)?),
                None => None,
            },
        ))
    } else if raw.contains("w") {
        let split = raw.split("w").collect::<Vec<&str>>();
        let minor_version = split.get(1).ok_or(Error::InvalidMinecraftVersion)?;
        Ok(MinecraftVersion::Snapshot(
            split
                .first()
                .ok_or(Error::InvalidMinecraftVersion)?
                .parse()
                .map_err(|_| Error::InvalidMinecraftVersion)?,
            (minor_version[..2])
                .parse()
                .map_err(|_| Error::InvalidMinecraftVersion)?,
            (minor_version[2..]).to_string(),
        ))
    } else {
        Ok(MinecraftVersion::Unknown(raw.to_string()))
    }
}
