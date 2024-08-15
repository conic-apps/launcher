/*
 * Amethyst Launcher Core
 * Copyright (C) 2023 Broken-Deer <old_driver__@outlook.com> and contributors
 *
 * This program is free software, you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fs::File;
use std::path::Path;
use std::str::FromStr;
use std::{ffi::OsStr, io::Read};

use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toml::Table;
use zip::ZipArchive;

use super::{Parse, ResolvedAuthorInfo, ResolvedDepends, ResolvedMod};
use crate::utils::unzip::filter_entries;

/// Represent the forge `mcmod.info` format.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeModMcmodInfo {
    pub mod_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub mcversion: Option<String>,
    pub url: Option<String>,
    pub update_url: Option<String>,
    pub update_json: Option<String>,
    pub author_list: Option<Vec<String>>,
    pub credits: Option<String>,
    pub logo_file: Option<String>,
    pub screenshots: Option<Vec<String>>,
    pub parrent: Option<String>,
    pub use_dependency_information: Option<bool>,
    pub required_mods: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub dependants: Option<Vec<String>>,
}

impl Parse for ForgeModMcmodInfo {
    fn parse(self) -> ResolvedMod {
        ResolvedMod {
            name: match self.name {
                Some(v) => v,
                None => match self.mod_id {
                    Some(v) => v,
                    None => "".to_string(),
                },
            },
            description: self.description,
            authors: {
                match self.author_list {
                    None => vec![],
                    Some(v) => v
                        .into_iter()
                        .map(|v| ResolvedAuthorInfo {
                            name: v,
                            contact: None,
                        })
                        .collect::<Vec<_>>(),
                }
            },
            version: self.version,
            icon: self.logo_file,
            license: None,
            depends: {
                match self.mcversion {
                    Some(v) => ResolvedDepends {
                        minecraft: Some(Value::String(v)),
                        java: None,
                        mod_loader: None,
                    },
                    None => ResolvedDepends {
                        minecraft: None,
                        java: None,
                        mod_loader: None,
                    },
                }
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeModTOMLMod {
    pub mod_id: Option<String>,
    pub description: Option<String>,
    pub display_name: Option<String>,
    pub version: Option<String>,
}

/// This file defines the metadata of your mod. Its information may be viewed by users from the main
/// screen of the game through the Mods button. A single info file can describe several mods.
///
/// The `mods.toml` file is formatted as TOML, the example mods.toml file in the MDK provides
/// comments explaining the contents of the file. It should be stored as
/// src/main/resources/META-INF/mods.toml. A basic mods.toml, describing one mod, may look like this:
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeModTOMLData {
    pub update_jsonurl: Option<String>,
    pub display_url: Option<String>,
    pub logo_file: Option<String>,
    pub credits: Option<String>,
    pub authors: Option<String>,
    pub dependencies: Option<Table>,
    pub mod_loader: Option<String>,
    pub loader_version: Option<String>,
    pub issue_tracker_url: Option<String>,
    pub mods: Option<Vec<ForgeModTOMLMod>>,
    pub mod_id: Option<String>,
    pub description: Option<String>,
    pub display_name: Option<String>,
    pub version: Option<String>,
}

impl FromStr for ForgeModTOMLData {
    type Err = anyhow::Error;
    fn from_str(str: &str) -> Result<ForgeModTOMLData> {
        let raw: ForgeModTOMLData = toml::from_str(str)?;

        match raw.mods.clone() {
            None => Ok(raw),
            Some(v) => {
                let mod_info = v.first();
                match mod_info {
                    None => Ok(raw),
                    Some(mod_info) => Ok(ForgeModTOMLData {
                        mod_id: mod_info.mod_id.clone(),
                        description: mod_info.description.clone(),
                        display_name: mod_info.display_name.clone(),
                        version: mod_info.version.clone(),
                        ..raw
                    }),
                }
            }
        }
    }
}

impl Parse for ForgeModTOMLData {
    fn parse(self) -> ResolvedMod {
        ResolvedMod {
            name: match self.display_name {
                Some(v) => v,
                None => match self.mod_id {
                    Some(v) => v,
                    None => "".to_string(),
                },
            },
            description: self.description,
            authors: {
                match self.authors {
                    None => vec![],
                    Some(v) => vec![ResolvedAuthorInfo {
                        name: v,
                        contact: None,
                    }],
                }
            },
            version: self.version,
            icon: self.logo_file,
            license: None,
            depends: {
                ResolvedDepends {
                    minecraft: None,
                    java: None,
                    mod_loader: None,
                }
            },
        }
    }
}

impl ForgeModMcmodInfo {
    pub fn from_info_file(file_content: &str) -> Result<ForgeModMcmodInfo> {
        let file_content = file_content
            .trim_start_matches('\u{feff}')
            .replace("\n\n", "\\n")
            .replace("\n", "");
        let info: Vec<ForgeModMcmodInfo> = serde_json::from_str(&file_content)?;
        Ok(info
            .first()
            .ok_or(anyhow::Error::new(std::io::Error::from(
                std::io::ErrorKind::NotFound,
            )))?
            .clone())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawManifest {
    pub manifest_version: Option<String>,
    pub tweak_order: Option<String>,
    pub tweak_version: Option<String>,
    pub tweak_meta_file: Option<String>,
    pub tweak_name: Option<String>,
    pub tweak_author: Option<String>,
    pub tweak_class: Option<String>,
    pub main_class: Option<String>,
}

/// The metadata inferred from manifest
pub struct ManifestMetadata {
    pub mod_id: Option<String>,
    pub name: Option<String>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub url: Option<String>,
}

impl FromStr for ManifestMetadata {
    type Err = anyhow::Error;
    fn from_str(str: &str) -> Result<ManifestMetadata> {
        let str = str
            .to_string()
            .lines()
            .map(|line| {
                let regex = Regex::new(r"\:").unwrap();
                if regex.is_match(line) {
                    let line = line.replace(": ", ":");
                    let kv = line.split(":").collect::<Vec<&str>>();
                    #[allow(clippy::get_first)]
                    let key = kv.get(0).unwrap().replace("-", "").to_string();
                    let value = kv.get(1).unwrap();
                    format!("{}=\"{}\"", key, value)
                } else {
                    "".to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        let raw: RawManifest = toml::from_str(&str)?;
        let mod_id = raw.tweak_meta_file.map(|v| v.replace(".json", ""));
        Ok(ManifestMetadata {
            name: raw.tweak_name,
            mod_id,
            authors: raw
                .tweak_author
                .map(|v| v.split(",").map(|v| v.to_string()).collect()),
            description: None,
            url: None,
        })
    }
}

impl Parse for ManifestMetadata {
    fn parse(self) -> ResolvedMod {
        ResolvedMod {
            name: match self.name {
                Some(v) => v,
                None => match self.mod_id {
                    Some(v) => v,
                    None => "".to_string(),
                },
            },
            description: self.description,
            authors: {
                match self.authors {
                    None => vec![],
                    Some(v) => v
                        .into_iter()
                        .map(|v| ResolvedAuthorInfo {
                            name: v,
                            contact: None,
                        })
                        .collect::<Vec<_>>(),
                }
            },
            version: None,
            icon: None,
            license: None,
            depends: {
                ResolvedDepends {
                    minecraft: None,
                    java: None,
                    mod_loader: None,
                }
            },
        }
    }
}

pub struct ResolvedForgeMod {
    pub name: String,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub version: Option<String>,
    pub logo_file: Option<String>,
}

pub fn parse_mod<P: AsRef<Path>>(path: P) -> Result<ResolvedMod> {
    let mod_file = File::open(path)?;
    let mut mod_file_archive = ZipArchive::new(mod_file)?;
    parse_mod_ziparchive(&mut mod_file_archive)
}

pub fn parse_mod_ziparchive(archive: &mut ZipArchive<File>) -> Result<ResolvedMod> {
    let target_entries = vec![
        "cccmod.info".to_string(),
        "mcmod.info".to_string(),
        "neimod.info".to_string(),
        "META-INF/mods.toml".to_string(),
        "META-INF/MANIFEST.MF".to_string(),
    ];
    let entries = filter_entries(archive, &target_entries);
    let mut result = if let Some(entry) = entries.get("mcmod.info") {
        let file_content = String::from_utf8(entry.content.clone())?;
        ForgeModMcmodInfo::from_info_file(&file_content)?.parse()
    } else if let Some(entry) = entries.get("neimod.info") {
        let file_content = String::from_utf8(entry.content.clone())?;
        ForgeModMcmodInfo::from_info_file(&file_content)?.parse()
    } else if let Some(entry) = entries.get("cccmod.info") {
        let file_content = String::from_utf8(entry.content.clone())?;
        ForgeModMcmodInfo::from_info_file(&file_content)?.parse()
    } else if let Some(entry) = entries.get("META-INF/mods.toml") {
        let file_content = String::from_utf8(entry.content.clone())?;
        ForgeModTOMLData::from_str(&file_content)?.parse()
    } else if let Some(entry) = entries.get("META-INF/MANIFEST.MF") {
        let file_content = String::from_utf8(entry.content.clone())?;
        ManifestMetadata::from_str(&file_content)?.parse()
    } else {
        return Err(anyhow::Error::new(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        )));
    };
    fn parse_icon(archive: &mut ZipArchive<File>, icon_path: String) -> Result<String> {
        let mut buf = Vec::new();
        archive.by_name(&icon_path)?.read_to_end(&mut buf)?;
        Ok(format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD_NO_PAD.encode(buf)
        ))
    }
    result.icon = match result.icon {
        None => None,
        Some(icon_path) => Some(parse_icon(archive, icon_path)?),
    };
    Ok(result)
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
        println!("{:?}", path);
        let resolved = match parse_mod(path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        result.push(resolved);
    }
    Ok(result)
}
