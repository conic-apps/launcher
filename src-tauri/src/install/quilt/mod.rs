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

use serde::{Deserialize, Serialize};
use tauri_plugin_http::reqwest;

use crate::{folder::MinecraftLocation, version::Version, HTTP_CLIENT};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuiltArtifactVersion {
    separator: String,
    build: u32,

    /// e.g. "org.quiltmc.quilt-loader:0.16.1"
    maven: String,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltVersionHashed {
    pub maven: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltVersionIntermediary {
    pub maven: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiltLibraries {
    pub client: Vec<QuiltLibrary>,
    pub common: Vec<QuiltLibrary>,
    pub server: Vec<QuiltLibrary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltLauncherMeta {
    pub version: u32,
    pub libraries: QuiltLibraries,
    pub main_class: QuiltMainClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltMainClass {
    pub client: Option<String>,
    pub server: Option<String>,
    pub server_launcher: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltVersion {
    pub loader: QuiltArtifactVersion,
    pub hashed: QuiltVersionHashed,
    pub intermediary: QuiltVersionIntermediary,
    pub launcher_meta: QuiltLauncherMeta,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuiltVersionList(Vec<QuiltVersion>);
impl QuiltVersionList {
    pub async fn new(mcversion: &str) -> anyhow::Result<Self> {
        let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mcversion}");
        let response = HTTP_CLIENT.get().unwrap().get(url).send().await?;
        Ok(response.json().await?)
    }
}

pub async fn install(
    mcversion: &str,
    quilt_version: &str,
    minecraft: MinecraftLocation,
) -> anyhow::Result<()> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{mcversion}/{quilt_version}/profile/json"
    );
    let response = reqwest::get(url).await.unwrap();
    let quilt_version_json: Version = response.json().await.unwrap();
    let version_name = quilt_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    // let libraries = quilt_version.libraries.clone().unwrap();
    // let hashed = libraries.iter().find(|l| match l["name"].as_str() {
    //     None => false,
    //     Some(name) => name.starts_with("org.quiltmc:hashed"),
    // });

    tokio::fs::create_dir_all(json_path.parent().unwrap()).await?;
    tokio::fs::write(
        json_path,
        serde_json::to_string_pretty(&quilt_version_json).unwrap(),
    )
    .await?;
    Ok(())
}
