// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{io::BufRead, path::PathBuf, process::Stdio};

use folder::DATA_LOCATION;
use futures::AsyncWriteExt;
use log::{error, info, trace};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

pub async fn get_neoforge_version_list() -> Result<Vec<String>> {
    let legacy_versions = HTTP_CLIENT
        .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge")
        .send()
        .await?
        .json::<Value>()
        .await?["versions"]
        .clone();
    let legacy_versions = serde_json::from_value::<Vec<String>>(legacy_versions)?;
    let modern_versions = HTTP_CLIENT
        .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge")
        .send()
        .await?
        .json::<Value>()
        .await?["versions"]
        .clone();
    let mut modern_versions = serde_json::from_value::<Vec<String>>(modern_versions)?;
    modern_versions.extend(legacy_versions);
    Ok(modern_versions)
}

/// Installs the specified version of Neoforge.
///
/// Downloads the installer, runs it using the bundled Java Runtime,
/// and then cleans up the temporary installer file.
///
/// # Arguments
/// * `install_dir` - The target directory where the client will be installed.
/// * `neoforge_version` - The version of Neoforge to install.
///
/// # Returns
/// * `Ok(())` on successful installation.
/// * `Err(Error)` if installation fails.
pub async fn install(install_dir: &PathBuf, neoforge_version: &str) -> Result<()> {
    info!("Start downloading the neoforge installer");
    let installer_path = download_installer(neoforge_version).await?;
    let java = "/usr/bin/java"; // TODO: Use config file
    info!("Running installer");

    let mut command = std::process::Command::new(java)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()?;

    let out = command
        .stdout
        .take()
        .ok_or(Error::NeoforgeInstallerFailed)?;
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    let pid = command.id();

    loop {
        buf.clear();
        let size = out.read_line(&mut buf)?;
        if size == 0 {
            break;
        }
        let line = buf.trim();
        println!("{:#?}", line);
        if line.contains("Successfully installed client into launcher") {
            success = true;
            info!("Successfully ran the neoforge installer");
        } else {
            trace!("[{pid}] {line}");
            println!("[{pid}] {buf}");
        }
    }

    let output = command.wait_with_output()?;
    async_fs::remove_file(installer_path).await?;
    if !success || !output.status.success() {
        error!("Failed to ran neoforge installer");
        return Err(Error::NeoforgeInstallerFailed);
    }
    Ok(())
}

/// Downloads the Neoforge installer JAR for the given version.
///
/// # Arguments
/// * `neoforge_version` - The version to download.
///
/// # Returns
/// * `Ok(PathBuf)` containing the path to the downloaded installer.
/// * `Err(Error)` if downloading fails.
pub async fn download_installer(neoforge_version: &str) -> Result<PathBuf> {
    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar"
    );
    info!("The installer url is: {installer_url}");

    let installer_path = DATA_LOCATION
        .temp
        .join(format!("{}.jar", uuid::Uuid::new_v4()));
    if let Some(parent) = installer_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }

    let mut file = async_fs::File::create(&installer_path).await?;
    let response = HTTP_CLIENT
        .get(installer_url)
        .send()
        .await?
        .error_for_status()?;
    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}
