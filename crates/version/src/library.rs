// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use platform::{OsFamily, PLATFORM_INFO};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::checks::check_allowed;
use crate::error::*;

pub fn resolve_libraries(libraries: Vec<Value>) -> Result<Vec<ResolvedLibrary>> {
    let mut result = Vec::new();
    for library in libraries {
        if library["clientreq"].as_bool() == Some(false) {
            continue;
        }
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
    let os_family_normalized = match PLATFORM_INFO.os_family {
        OsFamily::Windows => "windows",
        OsFamily::Linux => "linux",
        OsFamily::Macos => "osx",
    };
    let classifier_key = library["natives"]
        .as_object()?
        .get(os_family_normalized)?
        .as_str()?
        .replace("${arch}", "64");
    if let Some(classifier) = library["downloads"]["classifiers"]
        .get(&classifier_key)
        .and_then(|v| v.as_object())
        && let Some(url) = classifier.get("url").and_then(|v| v.as_str())
        && let Some(path) = classifier.get("path").and_then(|v| v.as_str())
    {
        return Some(ResolvedLibrary::Native(LibraryDownloadInfo {
            sha1: classifier
                .get("sha1")
                .and_then(|sha1| sha1.as_str())
                .map(|sha1| sha1.to_string()),
            size: classifier.get("size").and_then(|v| v.as_u64()),
            url: url.to_string(),
            path: path.to_string(),
        }));
    }
    // Legacy loader jsons carry no `downloads` metadata; the native artifact
    // follows the plain maven layout with the selected classifier appended.
    let coordinate = library["name"].as_str()?;
    let coordinate: Vec<&str> = coordinate.split(":").collect();
    if coordinate.len() != 3 {
        return None;
    }
    #[allow(clippy::get_first)]
    let package = coordinate.first()?.replace(".", "/");
    let name = *coordinate.get(1)?;
    let version = *coordinate.get(2)?;
    let base_url = library["url"]
        .as_str()
        .unwrap_or("https://libraries.minecraft.net/");
    let file_name = format!("{name}-{version}-{classifier_key}");
    Some(ResolvedLibrary::Native(LibraryDownloadInfo {
        sha1: None,
        size: None,
        url: format!("{base_url}{package}/{name}/{version}/{file_name}.jar"),
        path: format!("{package}/{name}/{version}/{file_name}.jar"),
    }))
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

    let base_url = library["url"]
        .as_str()
        .unwrap_or("https://libraries.minecraft.net/");
    // Forge publishes its own artifact on its maven repository with the
    // "-universal" classifier, but installs it locally under the plain file
    // name. Keep the local path plain so it matches both the installed file
    // and the classpath, and only classify the download URL. Skip libraries
    // whose coordinate already carries a classifier to avoid doubling it.
    let mut artifact = format!("{name}-{version}");
    if package == "net/minecraftforge" && *name == "forge" && !artifact.ends_with("-universal") {
        artifact.push_str("-universal");
    }
    let path = format!("{package}/{name}/{version}/{name}-{version}.jar");
    Ok(ResolvedLibrary::Common(LibraryDownloadInfo {
        sha1: None,
        size: None,
        url: format!("{base_url}{package}/{name}/{version}/{artifact}.jar"),
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
