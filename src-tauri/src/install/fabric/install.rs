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

use std::{io::BufRead, path::PathBuf, process::Stdio};

use anyhow::Result;
use tokio::io::AsyncWriteExt;

use crate::{DATA_LOCATION, HTTP_CLIENT};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FabricInstaller {
    pub url: String,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FabricInstallers(Vec<FabricInstaller>);

impl FabricInstallers {
    pub async fn new() -> anyhow::Result<Self> {
        anyhow::Result::Ok(
            HTTP_CLIENT
                .get()
                .unwrap()
                .get("https://meta.fabricmc.net/v2/versions/installer")
                .send()
                .await?
                .json()
                .await?,
        )
    }
    pub async fn download_latest(
        &self,
        file_location: &PathBuf,
    ) -> anyhow::Result<(), anyhow::Error> {
        let latest_installer = self
            .0
            .first()
            .ok_or(anyhow::Error::msg("Bad fabric installer list"))?;
        tokio::fs::create_dir_all(
            file_location
                .parent()
                .ok_or(anyhow::Error::msg("Unknown Error"))?,
        )
        .await?;
        let mut file = tokio::fs::File::create(file_location).await?;
        let response = HTTP_CLIENT
            .get()
            .unwrap()
            .get(latest_installer.url.clone())
            .send()
            .await?;
        let src = response.bytes().await?;
        file.write_all(&src).await?;
        anyhow::Result::Ok(())
    }
}

/// fabric-installer.jar arguments
pub struct FabricInstallOptions {
    pub mcversion: String,
    pub install_dir: PathBuf,
    pub loader: String,
}

/// Generate the fabric version JSON file to disk according to yarn and loader.
///
/// ### Arguments
///
/// * `loader` - The fabric loader version.
/// * `minecraft_location` - The minecraft location.
/// * `options` - The install options.
///
/// ### Example
///
/// ```rust
/// use aml_core::install::fabric::install::install_fabric;
/// use aml_core::core::folder::MinecraftLocation;
/// use aml_core::install::fabric::FabricLoaderArtifact;
///
/// async fn fn_name() {
///     let loader = FabricLoaderArtifact::new("1.19.4", "xxx").await; // xxx is your fabric loader version
///     let minecraft_location = MinecraftLocation::new("test");
///     let options = None;
///     install_fabric(loader.unwrap(), minecraft_location, options).await;
/// }
/// ```
pub async fn install(options: FabricInstallOptions) -> Result<()> {
    let fabric_installers = FabricInstallers::new().await?;
    let fabric_installer_path = DATA_LOCATION
        .get()
        .unwrap()
        .temp
        .join("fabric-installer.jar");
    fabric_installers
        .download_latest(&fabric_installer_path)
        .await?;
    let java = DATA_LOCATION.get().unwrap().default_jre.clone();
    let mut command = std::process::Command::new(java)
        .arg("-jar")
        .arg(&fabric_installer_path)
        .arg("client")
        .arg("-dir")
        .arg(options.install_dir)
        .arg("-mcversion")
        .arg(options.mcversion)
        .arg("-loader")
        .arg(options.loader)
        .arg("-launcher")
        .arg("win32")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    println!("Running fabric installer");
    let out = command.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    while let Ok(_) = out.read_line(&mut buf) {
        if let Ok(Some(_)) = command.try_wait() {
            break;
        }
        println!("{}", buf);
    }
    let installer_output = command.wait_with_output().unwrap();
    if !installer_output.status.success() {
        tokio::fs::remove_file(fabric_installer_path).await?;
        return Err(anyhow::Error::msg("Fabric installer failed"));
    }
    tokio::fs::remove_file(fabric_installer_path).await?;
    Ok(())
}
