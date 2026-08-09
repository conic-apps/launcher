// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{Read, Seek},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::ZipArchive;

use super::{
    ModLoader, ResolvedAuthorInfo, ResolvedDepends, ResolvedMod, file_name_without_extension,
    open_nested_jar, read_entry, read_icon, strip_bom,
};
use crate::error::{Error, Result};

/// A file entry extracted from a mod archive.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub content: Vec<u8>,
}

/// Collect the requested entries from a zip archive into a map keyed by entry name.
///
/// Entries that are not present (or can't be read) are simply skipped, so the
/// caller can distinguish "missing" from "present" via `HashMap::get`.
pub fn filter_entries<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    entries: &[&str],
) -> HashMap<String, Entry> {
    let mut resolved_entries = HashMap::with_capacity(entries.len());
    for i in 0..zip.len() {
        let mut zip_file = match zip.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = zip_file.name().to_string();
        if entries.contains(&name.as_str()) {
            let mut content = Vec::new();
            if zip_file.read_to_end(&mut content).is_ok() {
                resolved_entries.insert(name.clone(), Entry { name, content });
            }
        }
    }
    resolved_entries
}

/// Represent the forge `mcmod.info` format.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeModMcmodInfo {
    #[serde(rename = "modid", alias = "modId")]
    pub mod_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub mcversion: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "updateUrl", alias = "updateURL")]
    pub update_url: Option<String>,
    #[serde(rename = "updateJSON", alias = "updateJson")]
    pub update_json: Option<String>,
    pub author: Option<String>,
    pub authors: Option<Vec<String>>,
    #[serde(rename = "authorList")]
    pub author_list: Option<Vec<String>>,
    pub credits: Option<String>,
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
    pub screenshots: Option<Vec<String>>,
    #[serde(rename = "parent", alias = "parrent")]
    pub parent: Option<String>,
    #[serde(rename = "useDependencyInformation")]
    pub use_dependency_information: Option<bool>,
    #[serde(rename = "requiredMods")]
    pub required_mods: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub dependants: Option<Vec<String>>,
}

impl ForgeModMcmodInfo {
    pub fn parse(self) -> ResolvedMod {
        let authors = self
            .author_list
            .into_iter()
            .flatten()
            .chain(self.authors.into_iter().flatten())
            .chain(self.author)
            .map(|v| ResolvedAuthorInfo {
                name: v,
                contact: None,
            })
            .collect::<Vec<_>>();
        ResolvedMod {
            name: self
                .name
                .unwrap_or_else(|| self.mod_id.clone().unwrap_or_default()),
            description: self.description,
            authors,
            version: self.version,
            icon: self.logo_file,
            license: None,
            depends: ResolvedDepends {
                minecraft: self.mcversion.map(Value::String),
                java: None,
                mod_loader: None,
            },
            loader: ModLoader::Forge,
            disabled: false,
            source: None,
            source_id: None,
            version_id: None,
        }
    }
}

/// Parse the content of an `mcmod.info` / `neimod.info` / `cccmod.info` file.
///
/// The content may be one of three shapes (xmcl / HMCL compatible):
/// - a JSON array of mods
/// - a `{ "modList": [...] }` wrapper
/// - a single mod object `{ "modid": ... }`
pub fn parse_mcmod_info(content: &str, transform_newlines: bool) -> Result<Vec<ForgeModMcmodInfo>> {
    let content = strip_bom(content);
    let content = if transform_newlines {
        content.replace("\n\n", "\\n").replace('\n', "")
    } else {
        content.to_string()
    };
    let value: Value = serde_json::from_str(&content)?;
    let infos: Vec<ForgeModMcmodInfo> = if let Some(array) = value.as_array() {
        serde_json::from_value(Value::Array(array.clone()))?
    } else if value.get("modList").and_then(|v| v.as_array()).is_some() {
        serde_json::from_value(value)?
    } else if value.get("modid").is_some() {
        vec![serde_json::from_value(value)?]
    } else {
        return Err(Error::ModParseFailed("malformed mcmod.info".to_string()));
    };
    Ok(infos)
}

/// This file defines the metadata of your mod. Its information may be viewed by users from the main
/// screen of the game through the Mods button. A single info file can describe several mods.
///
/// The `mods.toml` file is formatted as TOML. A single mods.toml can describe several mods (the
/// BuildCraft-style sub-modules), so this struct keeps the full `[[mods]]` list.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeModTOMLData {
    #[serde(rename = "modLoader")]
    pub mod_loader: Option<String>,
    #[serde(rename = "loaderVersion")]
    pub loader_version: Option<String>,
    pub license: Option<String>,
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
    pub credits: Option<String>,
    pub authors: Option<String>,
    #[serde(rename = "displayURL")]
    pub display_url: Option<String>,
    #[serde(rename = "issueTrackerURL")]
    pub issue_tracker_url: Option<String>,
    #[serde(rename = "dependencies")]
    pub dependencies: Option<toml::Table>,
    #[serde(rename = "mods")]
    pub mods: Option<Vec<ForgeModTOMLMod>>,
}

/// One `[[mods]]` entry of a `mods.toml` file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeModTOMLMod {
    #[serde(rename = "modId")]
    pub mod_id: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
    pub credits: Option<String>,
    pub authors: Option<String>,
    pub license: Option<String>,
    #[serde(rename = "displayURL")]
    pub display_url: Option<String>,
}

impl ForgeModTOMLMod {
    pub fn parse(
        self,
        root: &ForgeModTOMLData,
        jar_version: Option<&str>,
        loader: ModLoader,
    ) -> ResolvedMod {
        let depends = self.mods_toml_depends(root);
        let version = resolve_jar_version(self.version, jar_version);
        let authors = self
            .authors
            .or_else(|| root.authors.clone())
            .map(|v| {
                vec![ResolvedAuthorInfo {
                    name: v,
                    contact: None,
                }]
            })
            .unwrap_or_default();
        let icon = self.logo_file.clone().or_else(|| root.logo_file.clone());
        ResolvedMod {
            name: self
                .display_name
                .clone()
                .unwrap_or_else(|| self.mod_id.clone().unwrap_or_default()),
            description: self.description,
            authors,
            version,
            icon,
            license: self
                .license
                .clone()
                .or_else(|| root.license.clone())
                .map(|v| vec![v]),
            depends,
            loader,
            disabled: false,
            source: None,
            source_id: None,
            version_id: None,
        }
    }

    fn mods_toml_depends(&self, root: &ForgeModTOMLData) -> ResolvedDepends {
        let Some(deps) = root.dependencies.as_ref() else {
            return ResolvedDepends {
                minecraft: None,
                java: None,
                mod_loader: None,
            };
        };
        let Some(mod_id) = self.mod_id.as_deref() else {
            return ResolvedDepends {
                minecraft: None,
                java: None,
                mod_loader: None,
            };
        };
        let Some(entries) = deps.get(mod_id).and_then(|v| v.as_array()) else {
            return ResolvedDepends {
                minecraft: None,
                java: None,
                mod_loader: None,
            };
        };
        let mut minecraft = None;
        let mut java = None;
        let mut mod_loader = None;
        for entry in entries {
            let Some(table) = entry.as_table() else {
                continue;
            };
            let Some(dep_mod_id) = table.get("modId").and_then(|v| v.as_str()) else {
                continue;
            };
            let range = table.get("versionRange").and_then(|v| v.as_str());
            match dep_mod_id {
                "minecraft" => minecraft = range.map(|s| Value::String(s.to_string())),
                "java" => java = range.map(|s| Value::String(s.to_string())),
                "forge" | "neoforge" => mod_loader = range.map(|s| Value::String(s.to_string())),
                _ => (),
            }
        }
        ResolvedDepends {
            minecraft,
            java,
            mod_loader,
        }
    }
}

/// Resolve the `${file.jarVersion}` placeholder against the `Implementation-Version`
/// attribute of the manifest, like xmcl (forge.ts) and HMCL do.
fn resolve_jar_version(version: Option<String>, jar_version: Option<&str>) -> Option<String> {
    match version {
        Some(v) if v == "${file.jarVersion}" => jar_version.map(|s| s.to_string()).or(Some(v)),
        v => v,
    }
}

/// Parse a `mods.toml` / `neoforge.mods.toml` file.
pub fn parse_mods_toml(content: &str) -> Result<ForgeModTOMLData> {
    let data: ForgeModTOMLData = toml::from_str(strip_bom(content))
        .map_err(|e| Error::ModParseFailed(format!("mods.toml: {e}")))?;
    Ok(data)
}

/// The metadata inferred from a `META-INF/MANIFEST.MF` file.
#[derive(Debug, Clone, Default)]
pub struct ManifestMetadata {
    pub mod_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub url: Option<String>,
    /// The `Implementation-Version` attribute, used to resolve `${file.jarVersion}`.
    pub jar_version: Option<String>,
}

impl ManifestMetadata {
    pub fn parse(self) -> ResolvedMod {
        ResolvedMod {
            name: self
                .name
                .unwrap_or_else(|| self.mod_id.clone().unwrap_or_default()),
            description: self.description,
            authors: self
                .authors
                .into_iter()
                .flatten()
                .map(|v| ResolvedAuthorInfo {
                    name: v,
                    contact: None,
                })
                .collect::<Vec<_>>(),
            version: self.version,
            icon: None,
            license: None,
            depends: ResolvedDepends {
                minecraft: None,
                java: None,
                mod_loader: None,
            },
            loader: ModLoader::Forge,
            disabled: false,
            source: None,
            source_id: None,
            version_id: None,
        }
    }
}

/// Parse the content of a JAR `MANIFEST.MF` file into a map of attributes,
/// following the JAR file specification:
///
/// - continuation lines (starting with a single space) are joined to the
///   previous entry,
/// - every entry is split at the **first** `:` only, so values may contain
///   colons (e.g. `https://...`) without breaking,
/// - `key:value` without a space is handled.
pub fn parse_manifest(content: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();
    for line in content.lines() {
        if line.starts_with(' ') {
            // Continuation of the previous header.
            if current_key.is_some() {
                current_value.push_str(line.strip_prefix(' ').unwrap_or_default());
            }
            continue;
        }
        if let Some(key) = current_key.take() {
            result.insert(key, current_value.trim().to_string());
        }
        current_value = String::new();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            if !key.is_empty() {
                current_key = Some(key.to_string());
                current_value = value.to_string();
            }
        }
    }
    if let Some(key) = current_key.take() {
        result.insert(key, current_value.trim().to_string());
    }
    result
}

/// Build the [`ManifestMetadata`] from raw manifest attributes, optionally
/// reading the `TweakMetaFile` JSON from the archive to fill missing fields.
pub fn manifest_metadata<R: Read + Seek>(
    manifest: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
) -> ManifestMetadata {
    let mut metadata = ManifestMetadata {
        mod_id: manifest.get("TweakName").cloned(),
        name: manifest.get("TweakName").cloned(),
        version: manifest.get("TweakVersion").cloned(),
        authors: manifest
            .get("TweakAuthor")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
        jar_version: manifest.get("Implementation-Version").cloned(),
        ..Default::default()
    };

    // A FML coremod announces its plugin class in the manifest. Without an ASM
    // scan this is the best cheap fallback id/name we have.
    if metadata.mod_id.is_none()
        && let Some(class) = manifest.get("FMLCorePlugin")
    {
        let class = class.trim_end_matches(".class");
        let name = class.rsplit('.').next().unwrap_or(class);
        let name = name.trim_end_matches("Plugin").to_string();
        if !name.is_empty() {
            metadata.mod_id = Some(name.clone());
            metadata.name = Some(name);
        }
    }

    if let Some(file) = manifest.get("TweakMetaFile")
        && let Some(bytes) = read_entry(archive, &format!("META-INF/{file}"))
        && let Ok(content) = String::from_utf8(bytes)
        && let Ok(json) = serde_json::from_str::<Value>(strip_bom(&content))
    {
        if let Some(id) = non_empty(json.get("id")) {
            metadata.mod_id = Some(id.to_string());
        }
        if let Some(name) = non_empty(json.get("name")) {
            metadata.name = Some(name.to_string());
        }
        if let Some(version) = non_empty(json.get("version")) {
            metadata.version = Some(version.to_string());
        }
        if let Some(authors) = json.get("authors").and_then(Value::as_array) {
            let authors = authors
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            if !authors.is_empty() {
                metadata.authors = Some(authors);
            }
        }
        if let Some(description) = non_empty(json.get("description")) {
            metadata.description = Some(description.to_string());
        }
        if let Some(url) = non_empty(json.get("url")) {
            metadata.url = Some(url.to_string());
        }
    }
    metadata
}

/// Like xmcl, only a non-empty value overrides the manifest metadata.
fn non_empty(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// The `META-INF/jarjar/metadata.json` format describing embedded dependencies.
#[derive(Debug, Clone, Deserialize)]
struct JarInJarMetadata {
    jars: Vec<EmbeddedJar>,
}

#[derive(Debug, Clone, Deserialize)]
struct EmbeddedJar {
    path: String,
}

/// Parse `META-INF/jarjar/metadata.json` and resolve every embedded jar as a
/// (possibly nested) mod archive.
fn parse_jarjar<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Vec<ResolvedMod> {
    let Some(bytes) = read_entry(archive, "META-INF/jarjar/metadata.json") else {
        return Vec::new();
    };
    let Ok(content) = String::from_utf8(bytes) else {
        return Vec::new();
    };
    let Ok(metadata) = serde_json::from_str::<JarInJarMetadata>(strip_bom(&content)) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for jar in metadata.jars {
        if let Some(mut nested) = open_nested_jar(archive, &jar.path)
            && let Ok(mods) = super::parse_mod_archive(&mut nested)
        {
            result.extend(mods);
        }
    }
    result
}

pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<Vec<ResolvedMod>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(|_| Error::NotAModFile)?;
    parse_mod_archive(&mut archive)
}

pub fn parse_mod_archive<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<ResolvedMod>> {
    let entries = filter_entries(
        archive,
        &[
            "mcmod.info",
            "neimod.info",
            "cccmod.info",
            "META-INF/mods.toml",
            "META-INF/neoforge.mods.toml",
            "META-INF/MANIFEST.MF",
        ],
    );

    let manifest_attrs = entries
        .get("META-INF/MANIFEST.MF")
        .map(|e| parse_manifest(&String::from_utf8_lossy(&e.content)))
        .unwrap_or_default();
    let jar_version = manifest_attrs
        .get("Implementation-Version")
        .map(|s| s.as_str());

    // 1. mods.toml / neoforge.mods.toml takes the highest precedence.
    let toml_loader = if entries.contains_key("META-INF/neoforge.mods.toml") {
        Some(("META-INF/neoforge.mods.toml", ModLoader::NeoForge))
    } else if entries.contains_key("META-INF/mods.toml") {
        Some(("META-INF/mods.toml", ModLoader::Forge))
    } else {
        None
    };
    if let Some((toml_name, loader)) = toml_loader {
        let entry = entries.get(toml_name).expect("toml_name from entries");
        let content = String::from_utf8_lossy(&entry.content).into_owned();
        match parse_mods_toml(&content) {
            Ok(data) => {
                let mut result = Vec::new();
                if let Some(mods) = &data.mods {
                    for mod_info in mods {
                        let mut resolved = mod_info.clone().parse(&data, jar_version, loader);
                        resolved.icon =
                            resolved.icon.as_deref().and_then(|p| read_icon(archive, p));
                        result.push(resolved);
                    }
                }
                if !result.is_empty() {
                    result.extend(parse_jarjar(archive));
                    return Ok(result);
                }
                // A mods.toml with no mod declarations: fall through to the
                // legacy formats rather than treating it as a non-mod.
            }
            Err(e) => return Err(Error::ModParseFailed(format!("{toml_name}: {e}"))),
        }
    }

    // 2. Legacy mcmod.info files. `cccmod.info` / `neimod.info` need the
    //    newline-joining pre-processing, `mcmod.info` is plain JSON.
    let info_files = [
        ("mcmod.info", false),
        ("neimod.info", true),
        ("cccmod.info", true),
    ];
    for (info_name, transform) in info_files {
        if let Some(entry) = entries.get(info_name) {
            let content = String::from_utf8_lossy(&entry.content).into_owned();
            let infos = parse_mcmod_info(&content, transform)?;
            if infos.is_empty() {
                continue;
            }
            let mut result = infos
                .into_iter()
                .map(|info| {
                    let mut resolved = info.parse();
                    resolved.icon = resolved.icon.as_deref().and_then(|p| read_icon(archive, p));
                    resolved
                })
                .collect::<Vec<_>>();
            result.extend(parse_jarjar(archive));
            return Ok(result);
        }
    }

    // 3. A bare MANIFEST.MF (tweak mods / FML coremods).
    if !manifest_attrs.is_empty() {
        let metadata = manifest_metadata(&manifest_attrs, archive);
        if metadata.mod_id.is_some() {
            return Ok(vec![metadata.parse()]);
        }
    }

    Err(Error::NotAModFile)
}

pub fn parse_folder<S: AsRef<OsStr> + ?Sized>(folder: &S) -> Result<Vec<ResolvedMod>> {
    let folder = Path::new(folder);
    let entries = folder.read_dir()?;
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        match parse_mod(&path) {
            Ok(mods) => result.extend(mods),
            Err(e) => {
                log::warn!("Failed to parse mod {:?}: {e}", path);
                result.push(ResolvedMod::unrecognized(Some(std::ffi::OsStr::new(
                    &file_name_without_extension(&path),
                ))));
            }
        }
    }
    Ok(result)
}
