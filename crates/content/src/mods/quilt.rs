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

/// The `quilt_loader` field of `quilt.mod.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuiltLoaderData {
    pub group: String,
    pub id: String,
    pub version: String,
    /// Can be plain mod ids or `{ id, version }` objects (quilt-api style).
    pub provides: Option<Vec<Value>>,
    pub metadata: Option<QuiltMetadata>,
    pub entrypoints: Option<Value>,
    pub jars: Option<Vec<String>>,
    pub language_adapters: Option<Value>,
    pub depends: Option<Vec<Value>>,
    pub breaks: Option<Vec<Value>>,
    pub load_type: Option<String>,
    pub minecraft: Option<Value>,
}

/// The `quilt_loader.metadata` object of `quilt.mod.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuiltMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub contributors: Option<HashMap<String, String>>,
    pub contact: Option<HashMap<String, String>>,
    pub license: Option<Value>,
    pub icon: Option<ModIcon>,
    /// Some mods still put the icon at the top level instead of in `metadata`.
    pub fabric_icon: Option<ModIcon>,
}

/// Corresponds to the `quilt.mod.json` file in the mod archive.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuiltModMetadata {
    #[serde(rename = "schema_version")]
    pub schema_version: u8,
    #[serde(rename = "quilt_loader")]
    pub quilt_loader: QuiltLoaderData,
    pub mixin: Option<Value>,
}

fn parse_depends(depends: &Option<Vec<Value>>) -> (Option<Value>, Option<Value>, Option<Value>) {
    let Some(depends) = depends else {
        return (None, None, None);
    };
    let mut minecraft = None;
    let mut mod_loader = None;
    let mut java = None;
    for dep in depends {
        let (id, versions) = match dep {
            Value::String(id) => (id.clone(), None),
            Value::Object(map) => (
                map.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                map.get("versions").cloned(),
            ),
            _ => continue,
        };
        let versions = versions.unwrap_or(Value::String("*".to_string()));
        match id.as_str() {
            "minecraft" => minecraft = Some(versions),
            "quilt_loader" | "fabricloader" => mod_loader = Some(versions),
            "java" => java = Some(versions),
            _ => (),
        }
    }
    (minecraft, java, mod_loader)
}

impl QuiltModMetadata {
    pub fn parse<R: Read + Seek>(self, archive: &mut ZipArchive<R>) -> ResolvedMod {
        let loader = &self.quilt_loader;
        let metadata = loader.metadata.as_ref();
        let name = metadata
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| loader.id.clone());
        let authors = metadata
            .and_then(|m| m.contributors.as_ref())
            .map(|contributors| {
                contributors
                    .keys()
                    .map(|name| ResolvedAuthorInfo {
                        name: name.clone(),
                        contact: None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let license = metadata.and_then(|m| parse_license(&m.license));
        let icon = metadata
            .and_then(|m| m.icon.as_ref())
            .or_else(|| metadata.and_then(|m| m.fabric_icon.as_ref()))
            .and_then(|icon| read_icon(archive, icon.path()));
        let (minecraft, java, mod_loader) = parse_depends(&loader.depends);
        ResolvedMod {
            name,
            description: metadata.and_then(|m| m.description.clone()),
            version: Some(loader.version.clone()),
            depends: ResolvedDepends {
                minecraft,
                java,
                mod_loader,
            },
            authors,
            license,
            icon,
            loader: ModLoader::Quilt,
            disabled: false,
            source: None,
            source_id: None,
            version_id: None,
        }
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
        Value::Object(map) => map
            .get("name")
            .and_then(Value::as_str)
            .map(|s| vec![s.to_string()]),
        _ => None,
    }
}

pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<Vec<ResolvedMod>> {
    let mut archive =
        ZipArchive::new(std::fs::File::open(path)?).map_err(|_| Error::NotAModFile)?;
    parse_mod_archive(&mut archive)
}

pub fn parse_mod_archive<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<ResolvedMod>> {
    let Some(content) = super::read_entry(archive, "quilt.mod.json") else {
        return Err(Error::NotAModFile);
    };
    let Ok(content) = String::from_utf8(content) else {
        return Err(Error::NotAModFile);
    };
    let metadata: QuiltModMetadata = serde_json::from_str(&super::sanitize_json(&content))
        .map_err(|e| Error::ModParseFailed(format!("quilt.mod.json: {e}")))?;

    let mut result = vec![metadata.clone().parse(archive)];
    if let Some(jars) = &metadata.quilt_loader.jars {
        for jar in jars {
            if let Some(mut nested) = open_nested_jar(archive, jar)
                && let Ok(mods) = super::parse_mod_archive(&mut nested)
            {
                result.extend(mods);
            }
        }
    }
    Ok(result)
}
