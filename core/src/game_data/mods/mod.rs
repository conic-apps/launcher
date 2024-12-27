// Amethyst Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Mods Parser. It support `forge`, `fabric`, `quilt`, `rift`
//!
//! If you want to parse mods for a specific mod loader,
//! you should use `mod_parser::<loader>::parse()` or `mod_parser::<loader>::parse_folder()`,
//! they filter mods that don't fit the format
//!
//! Note: If you want to parse `rift` mods, you should use forge mod parser.
//!
//! # Example
//!
//! Parse simple fabric mod:
//!
//! ```
//! use crate::game_data::mods::fabric;
//! use crate::game_data::mods::Parse;
//! use crate::game_data::mods::fabric::FabricModMetadata;
//!
//! let metadata = FabricModMetadata::from_path("mock/fabricMod/fabric-carpet-1.20.jar").unwrap();
//! let mod_info = metadata.parse();
//! println!("{:#?}", mod_info);
//! ```
//!
//! Resolve all mods in the folder:
//!
//! ```
//! use crate::game_data::mods::fabric::parse_folder;
//!
//! let result = parse_folder("mock/fabricMod").unwrap();
//! println!("{:#?}", result);
//! ```

use std::{collections::HashMap, ffi::OsStr, path::Path};

use anyhow::Result;
use serde_json::Value;

pub mod fabric;
pub mod forge;
pub mod quilt;

pub trait Parse {
    fn parse(self) -> ResolvedMod;
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedMod {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub depends: ResolvedDepends,
    pub authors: Vec<ResolvedAuthorInfo>,
    pub license: Option<Vec<String>>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedDepends {
    pub minecraft: Option<Value>,
    pub java: Option<Value>,
    pub mod_loader: Option<Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResolvedAuthorInfo {
    pub name: String,
    pub contact: Option<HashMap<String, String>>,
}

/// Mods parser. It support `forge`, `fabric`, `quilt`, `rift`
///
/// It will parse the mod using a parser that is suitable for the mod
pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<ResolvedMod> {
    match quilt::parse_mod(&path) {
        Ok(v) => Ok(v),
        Err(_) => forge::parse_mod(path),
    }
}

pub fn parse_folder<S: AsRef<OsStr> + ?Sized>(folder: &S) -> Result<Vec<ResolvedMod>> {
    let folder = Path::new(folder).to_path_buf();
    let entries = folder.read_dir()?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        result.push(parse_mod(path)?);
    }
    Ok(result)
}
