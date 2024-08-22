// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{ffi::OsStr, fs, io::Read, path::Path};

use anyhow::Result;
use zip::ZipArchive;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackMetadata {
    pub description: String,
    pub pack_format: u8,
    #[serde(flatten)]
    pub other: Option<serde_json::Value>,
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

pub fn parse_resourcespack<P: AsRef<Path>>(_path: P) {}
