// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
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

/// The raw version JSON as provided by Minecraft.
///
/// Resolve it into a [`ResolvedVersion`](crate::ResolvedVersion) to launch the game.
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

/// A Minecraft version parsed from a version string into release, snapshot or unknown form.
#[derive(Clone, Debug, PartialEq, Serialize)]
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
        // The week is two digits; slicing a shorter tail would panic rather
        // than reject the input.
        if minor_version.len() < 2 {
            return Err(Error::InvalidMinecraftVersion);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> MinecraftVersion {
        MinecraftVersion::from_str(raw).expect("should parse")
    }

    #[test]
    fn a_dotted_version_is_a_release() {
        assert_eq!(parse("1.20.1"), MinecraftVersion::Release(1, 20, Some(1)));
        assert_eq!(parse("1.21"), MinecraftVersion::Release(1, 21, None));
    }

    #[test]
    fn a_w_version_is_a_snapshot() {
        assert_eq!(
            parse("24w14a"),
            MinecraftVersion::Snapshot(24, 14, "a".to_string())
        );
        // The snapshot letter is whatever trails the two week digits.
        assert_eq!(
            parse("25w14craftmine"),
            MinecraftVersion::Snapshot(25, 14, "craftmine".to_string())
        );
    }

    #[test]
    fn something_that_is_neither_is_kept_verbatim() {
        assert_eq!(
            parse("not-a-version"),
            MinecraftVersion::Unknown("not-a-version".to_string())
        );
    }

    #[test]
    fn a_malformed_version_is_an_error_not_a_panic() {
        assert!(MinecraftVersion::from_str("1.").is_err());
        assert!(MinecraftVersion::from_str("1.a").is_err());
        // Fewer than two week digits must be rejected, not sliced out of bounds.
        assert!(MinecraftVersion::from_str("24w").is_err());
        assert!(MinecraftVersion::from_str("24wa").is_err());
    }

    #[test]
    fn an_empty_string_falls_back_to_unknown() {
        assert_eq!(parse(""), MinecraftVersion::Unknown(String::new()));
    }
}
