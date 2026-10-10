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
    // Every entry here can be dropped without failing, and a library missing from
    // the classpath surfaces at launch as a `ClassNotFoundException` or an
    // `UnsatisfiedLinkError` with nothing in the launcher log to explain it. So
    // the reasons are counted and reported once, rather than being silent skips.
    let (declared, mut not_required) = (libraries.len(), 0);
    let mut disallowed = 0;
    for library in libraries {
        if library["clientreq"].as_bool() == Some(false) {
            not_required += 1;
            continue;
        }
        let rules = library["rules"].as_array();
        if let Some(rules) = rules
            && !check_allowed(rules.clone(), &[])
        {
            disallowed += 1;
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
    let resolved = result.len();
    if resolved == 0 && declared > 0 {
        log::warn!("no library at all could be resolved out of {declared} declared");
    } else {
        log::debug!(
            "resolved {resolved} of {declared} librar{} ({not_required} not required on this \
             platform, {disallowed} disallowed by their rules)",
            if declared == 1 { "y" } else { "ies" }
        );
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
    // A loader library with no `url` of its own is silently pointed at Mojang's
    // maven root, which for Fabric / Quilt / NeoForge is the wrong repository
    // entirely: the download then 404s against a host nobody would think to blame.
    let base_url = match library["url"].as_str() {
        Some(url) => url,
        None => {
            log::warn!(
                "the native library {coordinate:?} declares no maven root; falling back to \
                 the Mojang root, which is likely the wrong repository for it"
            );
            "https://libraries.minecraft.net/"
        }
    };
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

/// A mod loader `version.json` `url` is the maven root, not the artifact path.
/// For example:
///
/// ```text
/// "libraries": [
///     {
///       "name": "net.fabricmc:tiny-mappings-parser:0.3.0+build.17",
///       "url": "https://maven.fabricmc.net/"
///     },
///   ]
/// ```
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

    let base_url = match library["url"].as_str() {
        Some(url) => url,
        None => {
            log::warn!(
                "the library {} declares no maven root; falling back to the Mojang root, \
                 which is likely the wrong repository for it",
                library["name"]
            );
            "https://libraries.minecraft.net/"
        }
    };
    let artifact = format!("{name}-{version}");
    let path = format!("{package}/{name}/{version}/{artifact}.jar");
    Ok(ResolvedLibrary::Common(LibraryDownloadInfo {
        sha1: None,
        size: None,
        url: format!("{base_url}{path}"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn common(info: &ResolvedLibrary) -> &LibraryDownloadInfo {
        match info {
            ResolvedLibrary::Common(info) => info,
            ResolvedLibrary::Native(_) => panic!("expected a common library"),
        }
    }

    #[test]
    fn a_mod_loader_url_is_the_maven_root() {
        let library = json!({
            "name": "net.fabricmc:tiny-mappings-parser:0.3.0+build.17",
            "url": "https://maven.fabricmc.net/",
        });
        let resolved = resolve_modloader_libraries(&library).expect("should resolve");
        let info = common(&resolved);
        assert_eq!(
            info.url,
            "https://maven.fabricmc.net/net/fabricmc/tiny-mappings-parser/0.3.0+build.17/tiny-mappings-parser-0.3.0+build.17.jar"
        );
        assert_eq!(
            info.path,
            "net/fabricmc/tiny-mappings-parser/0.3.0+build.17/tiny-mappings-parser-0.3.0+build.17.jar"
        );
    }

    #[test]
    fn a_mod_loader_without_a_url_uses_the_mojang_root() {
        let library = json!({ "name": "example:demo:1.0" });
        let resolved = resolve_modloader_libraries(&library).expect("should resolve");
        assert_eq!(
            common(&resolved).url,
            "https://libraries.minecraft.net/example/demo/1.0/demo-1.0.jar"
        );
    }

    #[test]
    fn a_malformed_coordinate_is_rejected() {
        assert!(resolve_modloader_libraries(&json!({ "name": "example:demo" })).is_err());
        assert!(resolve_modloader_libraries(&json!({})).is_err());
    }

    #[test]
    fn a_common_library_uses_its_artifact_download() {
        let library = json!({
            "downloads": {
                "artifact": {
                    "sha1": "abc",
                    "size": 12,
                    "url": "https://example.com/demo.jar",
                    "path": "example/demo/1.0/demo-1.0.jar",
                }
            }
        });
        let resolved = resolve_common_libraries(&library)
            .expect("should not error")
            .expect("should resolve");
        let info = common(&resolved);
        assert_eq!(info.url, "https://example.com/demo.jar");
        assert_eq!(info.path, "example/demo/1.0/demo-1.0.jar");
        assert_eq!(info.sha1.as_deref(), Some("abc"));
    }

    #[test]
    fn a_library_without_an_artifact_is_not_common() {
        assert!(resolve_common_libraries(&json!({ "downloads": {} })).is_ok());
        assert!(
            resolve_common_libraries(&json!({ "downloads": {} }))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_library_the_rules_disallow_is_dropped() {
        let libraries = vec![json!({
            "name": "example:demo:1.0",
            "rules": [{ "action": "disallow" }],
        })];
        assert!(resolve_libraries(libraries).unwrap().is_empty());
    }

    #[test]
    fn a_library_the_client_does_not_need_is_dropped() {
        let libraries = vec![json!({
            "name": "example:demo:1.0",
            "clientreq": false,
        })];
        assert!(resolve_libraries(libraries).unwrap().is_empty());
    }

    #[test]
    fn a_legacy_library_without_downloads_still_resolves() {
        let libraries = vec![json!({ "name": "example:demo:1.0" })];
        let resolved = resolve_libraries(libraries).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(common(&resolved[0]).path, "example/demo/1.0/demo-1.0.jar");
    }
}
