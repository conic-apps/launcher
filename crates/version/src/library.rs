// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use platform::PLATFORM_INFO;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::checks::check_allowed;
use crate::error::*;

#[derive(Clone, Serialize)]
pub struct Libraries(Vec<Value>);

impl Libraries {
    pub(crate) fn new() -> Self {
        Self(vec![])
    }

    pub(crate) fn join(&mut self, libraries: Vec<Value>) {
        self.0.splice(0..0, libraries);
    }

    pub(crate) fn to_resolved(&self) -> Result<Vec<ResolvedLibrary>> {
        let mut result = Vec::new();
        for library in self.0.clone() {
            let rules = library["rules"].as_array();
            if let Some(rules) = rules
                && !check_allowed(rules.clone(), &[])
            {
                continue;
            }
            let classifiers = library["downloads"]["classifiers"].as_object();
            let natives = library["natives"].as_object();
            if let Some(classifiers) = classifiers
                && let Some(natives) = natives
            {
                let classifier_key = match natives[&PLATFORM_INFO.os_family.to_string()].as_str() {
                    None => continue,
                    Some(x) => x,
                };
                let classifier = match classifiers[classifier_key].as_object() {
                    None => continue,
                    Some(x) => x,
                };
                let url = match classifier["url"].as_str() {
                    Some(url) => url.to_string(),
                    None => continue,
                };
                let path = match classifier["path"].as_str() {
                    Some(path) => path.to_string(),
                    None => continue,
                };
                result.push(ResolvedLibrary {
                    download_info: LibraryDownloadInfo {
                        sha1: classifier["sha1"].as_str().map(|sha1| sha1.to_string()),
                        size: classifier["size"].as_u64(),
                        url,
                        path,
                    },
                    is_native_library: true,
                });
                continue;
            }
            // resolve common lib
            if library["downloads"]["artifact"].is_object() {
                result.push(ResolvedLibrary {
                    download_info: serde_json::from_value(
                        library["downloads"]["artifact"].clone(),
                    )?,
                    is_native_library: false,
                });
                continue;
            }
            // resolve mod loader
            let name = match library["name"].as_str() {
                None => continue,
                Some(x) => x,
            };
            let name: Vec<&str> = name.split(":").collect();
            if name.len() != 3 {
                continue;
            }
            #[allow(clippy::get_first)]
            let package = name
                .get(0)
                .ok_or(Error::InvalidVersionJson)?
                .replace(".", "/");
            let version = name.get(2).ok_or(Error::InvalidVersionJson)?;
            let name = name.get(1).ok_or(Error::InvalidVersionJson)?;

            // NOTE: URL in mod loader version.json is NOT include path
            // For example:
            // "libraries": [
            //     {
            //       "name": "net.fabricmc:tiny-mappings-parser:0.3.0+build.17",
            //       "url": "https://maven.fabricmc.net/"
            //     },
            //   ]
            let url = library["url"]
                .as_str()
                .unwrap_or("https://libraries.minecraft.net/");
            let path = format!("{package}/{name}/{version}/{name}-{version}.jar");
            result.push(ResolvedLibrary {
                download_info: LibraryDownloadInfo {
                    sha1: None,
                    size: None,
                    url: format!("{url}{path}"),
                    path,
                },
                is_native_library: false,
            });
        }
        Ok(result)
    }
}
#[derive(Clone, Deserialize, Serialize)]
pub struct NormalLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownloadInfo>,
}

#[derive(Clone, Serialize)]
pub struct ResolvedLibrary {
    pub download_info: LibraryDownloadInfo,
    pub is_native_library: bool,
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
