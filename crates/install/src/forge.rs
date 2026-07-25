// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The `forge` module contains functionality related to Forge installation and version management.
//!
//! This module re-exports the `install` function from the `install` submodule,
//! and exposes the `version_list` submodule for managing Forge versions.

use std::{
    collections::HashMap,
    ffi::OsString,
    io::BufRead,
    path::{Path, PathBuf},
    process::{Child, Stdio},
};

use config::download::DownloadConfig;
use download::{download_concurrent, progress::DownloadState};
use folder::{DATA_LOCATION, MinecraftLocation};
use futures::AsyncWriteExt;
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;

use platform::DELIMITER;
use version::{Version, resolve_libraries};
use zip::ZipArchive;

use crate::{error::*, vanilla::generate_libraries_downloads};

/// A list of Forge versions for a given Minecraft version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionList(HashMap<String, Vec<String>>);

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
    pub async fn new() -> Result<Self> {
        Ok(HTTP_CLIENT
            .get("https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json")
            .send()
            .await?
            .json::<Self>()
            .await?)
    }
}

/// Forge Install Bootstrapper - by bangbang93
/// [GitHub Repository](https://github.com/bangbang93/forge-install-bootstrapper)
///
/// Embedded JAR file used for bootstrapping Forge installation on newer Forge versions.
const FORGE_INSTALL_BOOTSTRAPPER_BANGBANG93: &[u8] =
    include_bytes!("./forge-install-bootstrapper(bangbang93).jar");
/// Forge Install Bootstrapper - by OakChaser
///
/// [GitHub Repository](https://github.com/conic-apps/forge-install-bootstrapper-legacy)
///
/// Embedded JAR file used for bootstrapping legacy Forge installation.
const FORGE_INSTALL_BOOTSTRAPPER_CONIC: &[u8] =
    include_bytes!("./forge-install-bootstrapper(conic).jar");

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
    minecraft_location: &MinecraftLocation,
    forge_version: &str,
    mcversion: &str,
) -> Result<()> {
    info!("Start downloading the forge installer");
    let installer_path = download_installer(mcversion, forge_version).await?;
    let _ = prefetch_installer_dependencies(minecraft_location, &installer_path).await;
    let bangbang93_bootstrapper_installation_result =
        try_bangbang93_bootstrapper(&minecraft_location.root, &installer_path).await;
    let conicmc_bootstrapper_installation_result =
        if let Err(Error::ForgeInstallerFailed) = bangbang93_bootstrapper_installation_result {
            Some(try_conicmc_bootstrapper(&minecraft_location.root, &installer_path).await)
        } else {
            None
        };
    async_fs::remove_file(installer_path).await?;
    merge_results(
        bangbang93_bootstrapper_installation_result,
        conicmc_bootstrapper_installation_result,
    )
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
pub async fn download_installer(mcversion: &str, forge_version: &str) -> Result<PathBuf> {
    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{mcversion}-{forge_version}/forge-{mcversion}-{forge_version}-installer.jar"
    );
    info!("The installer url is: {installer_url}");
    let installer_path = DATA_LOCATION
        .temp
        .join(format!("forge-installer-{forge_version}.jar"));
    if let Some(parent) = installer_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    let mut file = async_fs::File::create(&installer_path).await?;
    // TODO: This can also return progress to frontend
    let response = HTTP_CLIENT
        .get(installer_url)
        .send()
        .await?
        .error_for_status()?;
    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}

async fn prefetch_installer_dependencies(
    minecraft_location: &MinecraftLocation,
    installer_path: &Path,
) -> Result<()> {
    let file = std::fs::File::open(installer_path)?;
    let mut archive = ZipArchive::new(file)?;
    let version_file = archive.by_name("version.json")?;
    let version: Version = serde_json::from_reader(version_file)?;
    if let Some(libraries) = version.libraries {
        let libraries = resolve_libraries(libraries)?;
        let download_entries = generate_libraries_downloads(minecraft_location, &libraries);
        download_concurrent(
            download_entries,
            &DownloadState::default(), // TODO: return progress to frontend
            DownloadConfig::default(),
        )
        .await?;
    }
    Ok(())
}

async fn try_bangbang93_bootstrapper(install_dir: &Path, installer_path: &Path) -> Result<()> {
    info!("Trying Bangbang93 forge install bootstrapper");
    let bangbang93_bootstrapper_path =
        save_bootstrapper(FORGE_INSTALL_BOOTSTRAPPER_BANGBANG93).await?;
    let child = std::process::Command::new("/usr/bin/java")
        .arg("-cp")
        .arg(generate_classpath(
            &bangbang93_bootstrapper_path,
            installer_path,
        )?)
        .arg("com.bangbang93.ForgeInstaller")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()?;
    let result = wait_child(child);
    async_fs::remove_file(bangbang93_bootstrapper_path).await?;
    result
}

async fn try_conicmc_bootstrapper(install_dir: &Path, installer_path: &Path) -> Result<()> {
    info!("Trying ConicMC forge install bootstrapper");
    let conicmc_bootstrapper_path = save_bootstrapper(FORGE_INSTALL_BOOTSTRAPPER_CONIC).await?;
    let child = std::process::Command::new("/usr/bin/java")
        .arg("-cp")
        .arg(generate_classpath(
            &conicmc_bootstrapper_path,
            installer_path,
        )?)
        .arg("app.conicmc.Bootstrap")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()?;
    let result = wait_child(child);
    async_fs::remove_file(conicmc_bootstrapper_path).await?;
    result
}

fn wait_child(mut child: Child) -> Result<()> {
    let out = child.stdout.take().ok_or(Error::ForgeInstallerFailed)?;
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    let pid = child.id();
    loop {
        buf.clear();
        let size = out.read_line(&mut buf)?;
        if size == 0 {
            break;
        }
        let line = buf.trim();
        if line == "true" {
            success = true;
            info!("Successfully ran the forge installer");
            println!("Successfully ran the forge installer");
        } else {
            trace!("[{pid}] {line}");
            println!("[{pid}] {buf}");
        }
    }
    let output = child.wait_with_output()?;
    if !success || !output.status.success() {
        error!("Failed to run forge installer");
        return Err(Error::ForgeInstallerFailed);
    }
    Ok(())
}

async fn save_bootstrapper(data: &[u8]) -> Result<PathBuf> {
    let bootstrapper_path = DATA_LOCATION.temp.join("forge-install-bootstrapper.jar");
    async_fs::write(&bootstrapper_path, data).await?;
    Ok(bootstrapper_path)
}

fn generate_classpath(bootstrapper_path: &Path, installer_path: &Path) -> Result<OsString> {
    let mut result = bootstrapper_path.canonicalize()?.into_os_string();
    result.push(DELIMITER);
    result.push(installer_path.canonicalize()?.into_os_string());
    Ok(result)
}

fn merge_results(bangbang93_result: Result<()>, conicmc_result: Option<Result<()>>) -> Result<()> {
    if bangbang93_result.is_ok() {
        return Ok(());
    }
    if let Err(bangbang93_err) = &bangbang93_result
        && matches!(bangbang93_err, &Error::ForgeInstallerFailed)
    {
        if let Some(conicmc_result) = conicmc_result
            && conicmc_result.is_ok()
        {
            Ok(())
        } else {
            Err(Error::ForgeInstallerFailed)
        }
    } else {
        bangbang93_result
    }
}
