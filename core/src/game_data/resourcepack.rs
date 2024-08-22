// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{ffi::OsStr, fs, io::Read, path::Path};

use anyhow::{anyhow, Result};
use zip::ZipArchive;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackMetadata {
    pub description: String,
    pub pack_format: u8,
    #[serde(flatten)]
    pub other: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Resourcepack {
    pub metadata: PackMetadata,
    pub icon: String,
    pub name: String,
    pub r#type: ResourcepackType,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum ResourcepackType {
    #[serde(rename = "texture")]
    Texture,
    #[serde(rename = "data")]
    Data,
    #[serde(rename = "unknown")]
    Unknown,
}

pub fn get_metadata<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<PackMetadata> {
    let path = Path::new(s).to_path_buf();
    let metadata = if path.is_dir() {
        fs::read_to_string(path.join("pack.mcmeta"))?
    } else {
        let file = fs::File::open(path)?;
        let mut zip_archive = ZipArchive::new(file)?;
        let mut zip_file = zip_archive.by_name("pack.metadata")?;
        let mut buf = String::new();
        zip_file.read_to_string(&mut buf)?;
        buf
    };
    Ok(serde_json::from_str(&metadata)?)
}

pub fn parse_resourcepack<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<Resourcepack> {
    let path = Path::new(s).to_path_buf();
    let pack_type = if let Ok(metadata) = path.join("data").metadata() {
        if metadata.is_dir() {
            ResourcepackType::Data
        } else {
            return Err(anyhow!("Bad pack"));
        }
    } else if let Ok(metadata) = path.join("assets").metadata() {
        if metadata.is_dir() {
            ResourcepackType::Texture
        } else {
            return Err(anyhow!("Bad pack"));
        }
    } else {
        ResourcepackType::Unknown
    };
    Ok(Resourcepack {
        metadata: get_metadata(&s)?,
        icon: "".to_string(), // TODO: icon
        name: path
            .file_name()
            .ok_or(anyhow!("No File name"))?
            .to_string_lossy()
            .to_string(),
        r#type: pack_type,
    })
}
