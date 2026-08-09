// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    io::{Read, Seek},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::ZipArchive;

use super::{
    ModIcon, ModLoader, ResolvedAuthorInfo, ResolvedDepends, ResolvedMod, open_nested_jar,
    read_icon,
};
use crate::error::{Error, Result};

/// One entry of the `jars` array in `fabric.mod.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JarsEntry {
    pub file: String,
}

/// Corresponds to the `fabric.mod.json` file in the mod archive.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricModMetadata {
    pub schema_version: u8,
    pub id: String,
    pub version: String,
    pub provides: Option<Vec<String>>,
    pub environment: Option<String>,
    pub entrypoints: Option<Value>,
    pub jars: Option<Vec<JarsEntry>>,
    pub language_adapters: Option<Value>,
    pub mixins: Option<Value>,
    pub depends: Option<HashMap<String, Value>>,
    pub recommends: Option<HashMap<String, Value>>,
    pub suggests: Option<HashMap<String, Value>>,
    pub breaks: Option<HashMap<String, Value>>,
    pub conflicts: Option<HashMap<String, Value>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub contact: Option<Value>,
    pub authors: Option<Vec<Value>>,
    pub contributors: Option<Vec<Value>>,
    pub license: Option<Value>,
    pub icon: Option<ModIcon>,
    pub custom: Option<Value>,
}

fn parse_person(value: &Value) -> ResolvedAuthorInfo {
    match value {
        Value::String(name) => ResolvedAuthorInfo {
            name: name.clone(),
            contact: None,
        },
        Value::Object(map) => ResolvedAuthorInfo {
            name: map
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            contact: None,
        },
        _ => ResolvedAuthorInfo {
            name: String::new(),
            contact: None,
        },
    }
}

fn parse_license(license: &Option<Value>) -> Option<Vec<String>> {
    let license = license.as_ref()?;
    match license {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect(),
        ),
        _ => None,
    }
}

impl FabricModMetadata {
    pub fn parse<R: Read + Seek>(self, archive: &mut ZipArchive<R>) -> ResolvedMod {
        let name = self.name.clone().unwrap_or_else(|| self.id.clone());
        let mut minecraft = None;
        let mut mod_loader = None;
        let mut java = None;
        if let Some(depends) = &self.depends {
            for (dep_id, range) in depends {
                match dep_id.as_str() {
                    "minecraft" => minecraft = Some(range.clone()),
                    "fabricloader" => mod_loader = Some(range.clone()),
                    "java" => java = Some(range.clone()),
                    _ => (),
                }
            }
        }
        let authors = self
            .authors
            .as_ref()
            .map(|items| items.iter().map(parse_person).collect())
            .unwrap_or_default();
        let icon = self
            .icon
            .as_ref()
            .and_then(|icon| read_icon(archive, icon.path()));
        ResolvedMod {
            name,
            description: self.description,
            version: Some(self.version.clone()),
            depends: ResolvedDepends {
                minecraft,
                java,
                mod_loader,
            },
            authors,
            license: parse_license(&self.license),
            icon,
            loader: ModLoader::Fabric,
            disabled: false,
            source: None,
            source_id: None,
            version_id: None,
        }
    }
}

pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<Vec<ResolvedMod>> {
    let mut archive =
        ZipArchive::new(std::fs::File::open(path)?).map_err(|_| Error::NotAModFile)?;
    parse_mod_archive(&mut archive)
}

pub fn parse_mod_archive<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<ResolvedMod>> {
    let Some(content) = super::read_entry(archive, "fabric.mod.json") else {
        return Err(Error::NotAModFile);
    };
    let Ok(content) = String::from_utf8(content) else {
        return Err(Error::NotAModFile);
    };
    let metadata: FabricModMetadata = serde_json::from_str(&super::sanitize_json(&content))
        .map_err(|e| Error::ModParseFailed(format!("fabric.mod.json: {e}")))?;

    let mut result = vec![metadata.clone().parse(archive)];
    if let Some(jars) = &metadata.jars {
        for jar in jars {
            if let Some(mut nested) = open_nested_jar(archive, &jar.file)
                && let Ok(mods) = super::parse_mod_archive(&mut nested)
            {
                result.extend(mods);
            }
        }
    }
    Ok(result)
}
