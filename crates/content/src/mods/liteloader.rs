// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ModLoader, ResolvedDepends, ResolvedMod};
use crate::error::{Error, Result};

/// Corresponds to the `litemod.json` file in a LiteLoader mod archive.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteLoaderModMetadata {
    pub name: String,
    #[serde(default)]
    pub mcversion: Option<String>,
    #[serde(default)]
    pub revision: Option<Value>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub tweak_class: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub inject_at: Option<String>,
    pub required_apis: Option<Vec<String>>,
    pub class_transformer_classes: Option<Vec<String>>,
}

impl LiteLoaderModMetadata {
    pub fn parse(self) -> ResolvedMod {
        let version = self.version.or_else(|| {
            Some(format!(
                "{}:{}",
                self.mcversion.as_deref().unwrap_or_default(),
                self.revision
                    .as_ref()
                    .map_or(0, |v| v.as_u64().unwrap_or(0))
            ))
        });
        ResolvedMod {
            path: PathBuf::new(),
            name: self.name.clone(),
            description: self.description,
            version,
            depends: ResolvedDepends {
                minecraft: self.mcversion.clone().map(Value::String),
                java: None,
                mod_loader: None,
            },
            authors: self
                .author
                .map(|author| {
                    vec![super::ResolvedAuthorInfo {
                        name: author,
                        contact: None,
                    }]
                })
                .unwrap_or_default(),
            license: None,
            icon: None,
            loader: ModLoader::LiteLoader,
            disabled: false,
            embedded: false,
            source: None,
            source_id: None,
            version_id: None,
        }
    }
}

pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<Vec<ResolvedMod>> {
    let path = path.as_ref();
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(path)?).map_err(|_| Error::NotAModFile)?;
    let mut mods = parse_mod_archive(&mut archive)?;
    for mod_info in &mut mods {
        mod_info.path = path.to_path_buf();
    }
    Ok(mods)
}

pub fn parse_mod_archive<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<Vec<ResolvedMod>> {
    let Some(content) = super::read_entry(archive, "litemod.json") else {
        return Err(Error::NotAModFile);
    };
    let Ok(content) = String::from_utf8(content) else {
        return Err(Error::NotAModFile);
    };
    let metadata: LiteLoaderModMetadata = serde_json::from_str(&super::sanitize_json(&content))
        .map_err(|e| Error::ModParseFailed(format!("litemod.json: {e}")))?;
    Ok(vec![metadata.parse()])
}
