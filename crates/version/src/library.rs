// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use platform::PLATFORM_INFO;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::checks::check_allowed;
use crate::error::*;

pub(crate) fn resolve_libraries(libraries: Vec<Value>) -> Result<Vec<ResolvedLibrary>> {
    let mut result = Vec::new();
    for library in libraries {
        let rules = library["rules"].as_array();
        if let Some(rules) = rules
            && !check_allowed(rules.clone(), &[])
        {
            continue;
        }
        if let Some(native_library) = resolve_native_libraries(&library) {
            result.push(native_library);
        } else if let Some(common_library) = resolve_common_libraries(&library)? {
            result.push(common_library);
        } else {
            result.push(resolve_modloader_libraries(&library)?);
        }
    }
    Ok(result)
}

fn resolve_native_libraries(library: &Value) -> Option<ResolvedLibrary> {
    if let Some(classifiers) = library["downloads"]["classifiers"].as_object()
        && let Some(natives) = library["natives"].as_object()
        && let Some(classifier_key) = natives[&PLATFORM_INFO.os_family.to_string()].as_str()
        && let Some(classifier) = classifiers[classifier_key].as_object()
        && let Some(url) = classifier["url"].as_str()
        && let Some(path) = classifier["path"].as_str()
    {
        Some(ResolvedLibrary::Native(LibraryDownloadInfo {
            sha1: classifier["sha1"].as_str().map(|sha1| sha1.to_string()),
            size: classifier["size"].as_u64(),
            url: url.to_string(),
            path: path.to_string(),
        }))
    } else {
        None
    }
}
fn resolve_common_libraries(library: &Value) -> Result<Option<ResolvedLibrary>> {
    if library["downloads"]["artifact"].is_object() {
        Ok(Some(ResolvedLibrary::Common(serde_json::from_value(
            library["downloads"]["artifact"].clone(),
        )?)))
    } else {
        Ok(None)
    }
}

/// URL in mod loader version.json is NOT include path
/// For example:
/// "libraries": [
///     {
///       "name": "net.fabricmc:tiny-mappings-parser:0.3.0+build.17",
///       "url": "https://maven.fabricmc.net/"
///     },
///   ]
fn resolve_modloader_libraries(library: &Value) -> Result<ResolvedLibrary> {
    let name = library["name"].as_str().ok_or(Error::InvalidVersionJson)?;
    let name: Vec<&str> = name.split(":").collect();
    if name.len() != 3 {
        return Err(Error::InvalidVersionJson);
    }
    #[allow(clippy::get_first)]
    let package = name
        .get(0)
        .ok_or(Error::InvalidVersionJson)?
        .replace(".", "/");
    let version = name.get(2).ok_or(Error::InvalidVersionJson)?;
    let name = name.get(1).ok_or(Error::InvalidVersionJson)?;

    let url = library["url"]
        .as_str()
        .unwrap_or("https://libraries.minecraft.net/");
    let path = format!("{package}/{name}/{version}/{name}-{version}.jar");
    Ok(ResolvedLibrary::Common(LibraryDownloadInfo {
        sha1: None,
        size: None,
        url: format!("{url}{path}"),
        path,
    }))
}

#[derive(Clone, Deserialize, Serialize)]
pub struct NormalLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownloadInfo>,
}

#[derive(Clone, Serialize)]
pub enum ResolvedLibrary {
    Native(LibraryDownloadInfo),
    Common(LibraryDownloadInfo),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LibraryDownloadInfo {
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
    pub path: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct NativeLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownloadInfo>,
    pub classifiers: HashMap<String, LibraryDownloadInfo>,
    pub rules: Vec<Value>,
    pub extract: Value,
    pub natives: HashMap<String, String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PlatformSpecificLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownloadInfo>,
    pub rules: Vec<Value>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LegacyLibrary {
    pub name: String,
    pub url: Option<String>,
    pub clientreq: Option<bool>,
    pub serverreq: Option<bool>,
    pub checksums: Option<Vec<String>>,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Library {
    Normal(NormalLibrary),
    Native(NativeLibrary),
    PlatformSpecific(PlatformSpecificLibrary),
    Legacy(LegacyLibrary),
}
