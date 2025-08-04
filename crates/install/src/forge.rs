// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The `forge` module contains functionality related to Forge installation and version management.
//!
//! This module re-exports the `install` function from the `install` submodule,
//! and exposes the `version_list` submodule for managing Forge versions.

use std::{io::BufRead, path::PathBuf, process::Stdio};

use anyhow::Result;
use folder::DATA_LOCATION;
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;
use tauri_plugin_http::reqwest;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use platform::DELIMITER;

/// Represents a single Forge version entry returned from the version list API.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionListItem {
    pub _id: String,
    pub build: u32,
    pub __v: u32,
    pub version: String,
    pub modified: String,
    pub mcversion: String,
    pub files: Vec<ForgeInstallerFile>,
    pub branch: Option<Value>,
}

/// Represents a single installer file entry in a Forge version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeInstallerFile {
    pub format: String,
    pub category: String,
    pub hash: Option<String>,
}

/// A list of Forge versions for a given Minecraft version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionList(Vec<ForgeVersionListItem>);

impl ForgeVersionList {
    /// Fetches the Forge version list for a specified Minecraft version.
    ///
    /// # Arguments
    ///
    /// * `mcversion` - The target Minecraft version (e.g., "1.20.1").
    ///
    /// # Returns
    ///
    /// A `ForgeVersionList` containing all available Forge versions for the specified Minecraft version.
    pub async fn new(mcversion: &str) -> Result<Self> {
        Ok(reqwest::get(format!(
            "https://bmclapi2.bangbang93.com/forge/minecraft/{mcversion}"
        ))
        .await?
        .json::<Self>()
        .await?)
    }
}

/// Forge Install Bootstrapper - by bangbang93
/// [GitHub Repository](https://github.com/bangbang93/forge-install-bootstrapper)
///
/// Embedded JAR file used for bootstrapping Forge installation on newer Forge versions.
const FORGE_INSTALL_BOOTSTRAPPER: &[u8] = include_bytes!("./forge-install-bootstrapper.jar");

/// Installs the specified Forge version for the given Minecraft version into the target directory.
///
/// This function downloads the Forge installer, optionally uses the bootstrapper JAR for Forge
/// versions 25 and above, and executes the installer to install Forge.
///
/// # Arguments
///
/// * `install_dir` - The directory where Forge should be installed.
/// * `forge_version` - The Forge version string to install (e.g., "1.20.1-47.1.0").
/// * `mcversion` - The Minecraft version string associated with this Forge version.
///
/// # Errors
///
/// Returns an error if downloading the installer, writing files, or running the installer process fails.
///
/// # Notes
///
/// The function manages temporary files, logging progress and errors throughout the installation.
pub async fn install(
    install_dir: &PathBuf,
    forge_version: &str,
    mcversion: &str,
) -> anyhow::Result<()> {
    let splited_forge_version: Vec<_> = forge_version.split(".").collect();
    let bootstrapper = if splited_forge_version
        .first()
        .ok_or(anyhow::Error::msg("Error forge version"))?
        .parse::<usize>()?
        < 25
    {
        info!("Not using bootstrapper");
        None
    } else {
        info!("Using bootstrapper");
        Some(FORGE_INSTALL_BOOTSTRAPPER)
    };
    info!("Start downloading the forge installer");
    let installer_path = download_installer(mcversion, forge_version).await?;
    info!("Saving bootstrapper");
    let bootstrapper_path = DATA_LOCATION.temp.join("forge-install-bootstrapper.jar");
    if let Some(bootstrapper) = bootstrapper {
        tokio::fs::write(&bootstrapper_path, bootstrapper).await?;
    }
    let java = DATA_LOCATION.default_jre.clone();
    info!("Running installer");
    let mut command = match bootstrapper {
        Some(_) => std::process::Command::new(java)
            .arg("-cp")
            .arg(format!(
                "{}{}{}",
                bootstrapper_path.to_str().unwrap(),
                DELIMITER,
                installer_path.to_str().unwrap()
            ))
            .arg("com.bangbang93.ForgeInstaller")
            .arg(install_dir)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
        None => std::process::Command::new(java)
            .arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(install_dir)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    };
    let out = command.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    let pid = command.id();
    while out.read_line(&mut buf).is_ok() {
        if let Ok(Some(_)) = command.try_wait() {
            break;
        }
        if buf.ends_with("\ntrue\n") {
            success = true;
            info!("Successfully ran the forge installer")
        } else {
            let lines: Vec<_> = buf.split("\n").collect();
            if let Some(last) = lines.get(lines.len() - 2) {
                trace!("[{pid}] {last}");
            }
        }
    }
    let output = command.wait_with_output().unwrap();
    if (!success && bootstrapper.is_some()) || !output.status.success() {
        tokio::fs::remove_file(installer_path).await?;
        error!("Failed to run forge installer");
        return Err(anyhow::Error::msg("Failed to run forge installer"));
    }
    tokio::fs::remove_file(installer_path).await?;
    Ok(())
}

/// Downloads the Forge installer JAR for the specified Minecraft and Forge versions.
///
/// Saves the installer to a temporary file and returns the file path.
///
/// # Arguments
///
/// * `mcversion` - The Minecraft version string.
/// * `forge_version` - The Forge version string.
///
/// # Returns
///
/// A `PathBuf` pointing to the downloaded installer JAR.
///
/// # Errors
///
/// Returns an error if the download fails or the file cannot be written.
pub async fn download_installer(mcversion: &str, forge_version: &str) -> anyhow::Result<PathBuf> {
    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{mcversion}-{forge_version}/forge-{mcversion}-{forge_version}-installer.jar"
    );
    info!("The installer url is: {installer_url}");
    let installer_path = DATA_LOCATION.temp.join(format!("{}.jar", Uuid::new_v4()));
    tokio::fs::create_dir_all(
        installer_path
            .parent()
            .ok_or(anyhow::Error::msg("Unknown Error"))?,
    )
    .await?;
    let mut file = tokio::fs::File::create(&installer_path).await?;
    let response = HTTP_CLIENT.get(installer_url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::Error::msg("Forge website return error"));
    }
    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}
