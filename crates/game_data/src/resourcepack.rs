// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{ffi::OsStr, fs, io::Read, path::Path};

use base64::{Engine, engine::general_purpose};
use folder::DATA_LOCATION;
use serde_json::Value;
use tauri::command;
use uuid::Uuid;
use zip::ZipArchive;

use crate::error::*;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResourcePack {
    pub metadata: Value,
    pub icon: Option<String>,
    pub name: String,
}

fn get_metadata<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<Value> {
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

fn get_icon<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<String> {
    let path = Path::new(s).to_path_buf();
    let icon_bytes = if path.is_dir() {
        fs::read(path.join("pack.png"))?
    } else {
        let file = fs::File::open(path)?;
        let mut zip_archive = ZipArchive::new(file)?;
        let mut zip_file = zip_archive.by_name("pack.png")?;
        let mut buf = vec![];
        zip_file.read_to_end(&mut buf)?;
        buf
    };
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD_NO_PAD.encode(icon_bytes)
    ))
}

pub fn parse_resourcepack<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<ResourcePack> {
    let path = Path::new(s);
    Ok(ResourcePack {
        metadata: get_metadata(&s)?,
        icon: get_icon(&s).ok(),
        name: path
            .file_name()
            .ok_or(Error::BadPack)?
            .display()
            .to_string(),
    })
}

pub fn get_all_resourcepacks<P: AsRef<Path>>(
    resourcepacks_folder_path: P,
) -> Result<Vec<ResourcePack>> {
    Ok(fs::read_dir(resourcepacks_folder_path)?
        .flatten()
        .flat_map(|entry| parse_resourcepack(&entry.path()))
        .collect::<Vec<_>>())
}

#[command]
pub(crate) async fn cmd_get_all_resourcepacks(instance_id: Uuid) -> Result<Vec<ResourcePack>> {
    get_all_resourcepacks(
        DATA_LOCATION
            .get_instance_root(&instance_id)
            .join("resourcepacks"),
    )
}
