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

use std::str::FromStr;

use anyhow::{anyhow, Result};
use reqwest::Url;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::core::version::ResolvedLibrary;
use crate::core::Download;
use crate::core::{
    folder::MinecraftLocation,
    version::{self, AssetIndex, AssetIndexObject, ResolvedVersion, VersionManifest},
};
use crate::PLATFORM_INFO;

pub mod fabric;
pub mod forge;
// pub mod optifine;
pub mod quilt;

/// todo
pub struct NetworkOptions {
    pub use_proxy: bool,
    pub minecraft_remote: String,
    pub forge_remote: String,
    pub fabric_remote: String,
    pub optifine_remote: String,
    pub quilt_remote: String,
}

pub(crate) fn generate_libraries_downloads(
    libraries: &Vec<ResolvedLibrary>,
    minecraft_location: &MinecraftLocation,
) -> Vec<Download> {
    libraries
        .clone()
        .into_iter()
        .map(|library| Download {
            url: if library.is_native_library {
                println!("find native library url: {}", &library.download_info.url);
                library.download_info.url
            } else {
                format!(
                    "https://libraries.minecraft.net/{}",
                    library.download_info.path
                )
            },
            file: minecraft_location
                .libraries
                .join(library.download_info.path),
            sha1: library.download_info.sha1,
        })
        .collect()
}

pub(crate) async fn generate_assets_downloads(
    asset_index: AssetIndex,
    minecraft_location: &MinecraftLocation,
) -> Result<Vec<Download>> {
    let asset_index_url = Url::parse((&asset_index.url).as_ref())?;
    let asset_index_raw = reqwest::get(asset_index_url).await?.text().await?;
    let asset_index_json: Value = serde_json::from_str((&asset_index_raw).as_ref())?;
    let asset_index_object: AssetIndexObject =
        serde_json::from_value(asset_index_json["objects"].clone())?;
    let mut assets: Vec<_> = asset_index_object
        .into_iter()
        .map(|obj| Download {
            url: format!(
                "https://resources.download.minecraft.net/assets/{}/{}",
                &obj.1.hash[0..2],
                obj.1.hash
            ),
            file: minecraft_location
                .assets
                .join("objects")
                .join(&obj.1.hash[0..2])
                .join(&obj.1.hash),
            sha1: Some(obj.1.hash),
        })
        .collect();
    assets.push(Download {
        url: asset_index.url,
        file: minecraft_location
            .assets
            .join("indexes")
            .join(format!("{}.json", asset_index.id)),
        sha1: None,
    });
    Ok(assets)
}

/// check game integrity and try to repair files
///
/// This is usually done in situations where the integrity of the game is uncertain,
/// such as launching for the first time after installation
pub async fn generate_dependencies_downloads(
    version: ResolvedVersion,
    minecraft_location: MinecraftLocation,
) -> Result<()> {
    let mut download_list = Vec::new();

    download_list.extend(generate_libraries_downloads(
        &version.libraries,
        &minecraft_location,
    ));
    download_list.extend(
        generate_assets_downloads(version.asset_index.clone().unwrap(), &minecraft_location)
            .await?,
    );
    let log4j2 = generate_log4j2_configuration_download(&version, &minecraft_location);
    if let Ok(log4j2) = log4j2 {
        download_list.push(log4j2);
    }

    Ok(())
}

pub fn generate_log4j2_configuration_download(
    version: &ResolvedVersion,
    minecraft_location: &MinecraftLocation,
) -> Result<Download> {
    let logging = version.logging.clone().ok_or(anyhow!("No logging found"))?;
    let logging_client = logging
        .get("client")
        .ok_or(anyhow!("No logging client found"))?
        .clone();
    Ok(Download {
        url: logging_client.file.url,
        file: minecraft_location
            .get_version_root(version.id.clone())
            .join("log4j2.xml"),
        sha1: Some(logging_client.file.sha1),
    })
}

pub async fn generate_download_info(
    version_id: &str,
    minecraft_location: MinecraftLocation,
) -> Result<Vec<Download>> {
    let platform = PLATFORM_INFO.get().unwrap();
    let versions = VersionManifest::new().await?.versions;
    let version_metadata: Vec<_> = versions
        .into_iter()
        .filter(|v| v.id == version_id)
        .collect();
    if version_metadata.len() != 1 {
        panic!("Bad version manifest!!!")
    };
    let version_metadata = version_metadata.get(0).unwrap();

    let version_json_raw = reqwest::get(version_metadata.url.clone())
        .await?
        .text()
        .await?;
    let version = version::Version::from_str(&version_json_raw)?
        .parse(&minecraft_location, platform)
        .await?;
    let id = &version.id;

    let version_json_path = minecraft_location.versions.join(format!("{id}/{id}.json"));
    tokio::fs::create_dir_all(version_json_path.parent().unwrap()).await?;
    let mut file = tokio::fs::File::create(&version_json_path).await?;
    file.write_all(version_json_raw.as_bytes()).await?;

    let mut download_list = vec![];
    // download_list.push(Download {
    //     url: format!("https://download.mcbbs.net/version/{version_id}/client"),
    //     file: minecraft_location.versions.join(format!("{id}/{id}.jar")),
    //     sha1: None,
    // });
    let downloads = version
        .downloads
        .clone()
        .ok_or(anyhow!("No downloads found!"))?;
    let client = downloads.get("client").ok_or(anyhow!("No client found!"))?;

    download_list.push(Download {
        url: format!(
            "https://piston-data.mojang.com/v1/objects/{}/client.jar",
            client.sha1
        ),
        file: minecraft_location.versions.join(format!("{id}/{id}.jar")),
        sha1: Some(client.sha1.to_string()),
    });

    download_list.extend(generate_libraries_downloads(
        &version.libraries,
        &minecraft_location,
    ));
    download_list.extend(
        generate_assets_downloads(
            version
                .asset_index
                .clone()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?,
            &minecraft_location,
        )
        .await?,
    );
    let log4j2 = generate_log4j2_configuration_download(&version, &minecraft_location);
    if let Ok(log4j2) = log4j2 {
        download_list.push(log4j2);
    }
    Ok(download_list)
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::core::folder::MinecraftLocation;
//     use crate::core::HTTP_CLIENT;
//     #[tokio::test]
//     async fn test() {
//         let platform = PlatformInfo::new().await;
//         let downloads = generate_download_info("1.19.3", MinecraftLocation::new("test"), &platform)
//             .await
//             .unwrap();
//         for (index, download) in downloads.into_iter().enumerate() {
//             println!("{}", index);
//             let mut response = HTTP_CLIENT.get(download.url).send().await.unwrap();
//             tokio::fs::create_dir_all(download.file.parent().unwrap())
//                 .await
//                 .unwrap();
//             let mut file = tokio::fs::File::create(download.file).await.unwrap();
//             while let Some(chunk) = response.chunk().await.unwrap() {
//                 file.write_all(&chunk).await.unwrap();
//             }
//         }
//     }
// }
