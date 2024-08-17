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

use crate::{folder::MinecraftLocation, version::Version};
use tauri_plugin_http::reqwest;

pub async fn install(
    mcversion: &str,
    quilt_version: &str,
    minecraft: MinecraftLocation,
) -> anyhow::Result<()> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{mcversion}/{quilt_version}/profile/json"
    );
    let response = reqwest::get(url).await.unwrap();
    let fabric_version_json: Version = response.json().await.unwrap();
    let version_name = fabric_version_json.id.clone();
    let json_path = minecraft.get_version_json(&version_name);
    // let libraries = quilt_version.libraries.clone().unwrap();
    // let hashed = libraries.iter().find(|l| match l["name"].as_str() {
    //     None => false,
    //     Some(name) => name.starts_with("org.quiltmc:hashed"),
    // });

    tokio::fs::create_dir_all(json_path.parent().unwrap()).await?;
    tokio::fs::write(
        json_path,
        serde_json::to_string_pretty(&fabric_version_json).unwrap(),
    )
    .await?;
    Ok(())
}
