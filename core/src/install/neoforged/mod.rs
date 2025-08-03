// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{io::BufRead, path::PathBuf, process::Stdio};

use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{DATA_LOCATION, HTTP_CLIENT};

/// Represents the list of Neoforged versions.
#[derive(Deserialize, Serialize, Clone)]
pub struct NeoforgedVersionList {
    /// Whether the version is a snapshot.
    #[serde(rename = "isSnapshot")]
    pub is_snapshot: bool,

    /// The list of available Neoforged versions.
    pub versions: Vec<String>,
}

impl NeoforgedVersionList {
    /// Fetches the Neoforged version list from the remote API.
    ///
    /// # Returns
    /// * `Ok(Self)` on success.
    /// * `Err(anyhow::Error)` if the request fails or the data cannot be parsed.
    pub async fn new() -> anyhow::Result<Self> {
        Ok(HTTP_CLIENT
            .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge")
            .send()
            .await?
            .json()
            .await?)
    }
}

/// Installs the specified version of Neoforged.
///
/// Downloads the installer, runs it using the bundled Java Runtime,
/// and then cleans up the temporary installer file.
///
/// # Arguments
/// * `install_dir` - The target directory where the client will be installed.
/// * `neoforged_version` - The version of Neoforged to install.
///
/// # Returns
/// * `Ok(())` on successful installation.
/// * `Err(anyhow::Error)` if installation fails.
pub async fn install(install_dir: &PathBuf, neoforged_version: &str) -> anyhow::Result<()> {
    info!("Start downloading the neoforged installer");
    let installer_path = download_installer(neoforged_version).await?;
    let java = DATA_LOCATION.default_jre.clone();
    info!("Running installer");

    let mut command = std::process::Command::new(java)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let out = command.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    let pid = command.id();

    // Read output and check for success indicator
    while out.read_line(&mut buf).is_ok() {
        if let Ok(Some(_)) = command.try_wait() {
            break;
        }
        if buf.contains("Successfully installed client into launcher") {
            success = true;
            info!("Successfully ran the neoforged installer")
        } else {
            let lines: Vec<_> = buf.split('\n').collect();
            if let Some(last) = lines.get(lines.len() - 2) {
                trace!("[{}] {}", pid, last);
            }
        }
    }

    let output = command.wait_with_output().unwrap();
    if !success || !output.status.success() {
        tokio::fs::remove_file(installer_path).await?;
        error!("Failed to ran neoforged installer");
        return Err(anyhow::Error::msg("Failed to ran neoforged installer"));
    }

    tokio::fs::remove_file(installer_path).await?;
    Ok(())
}

/// Downloads the Neoforged installer JAR for the given version.
///
/// # Arguments
/// * `neoforged_version` - The version to download.
///
/// # Returns
/// * `Ok(PathBuf)` containing the path to the downloaded installer.
/// * `Err(anyhow::Error)` if downloading fails.
async fn download_installer(neoforged_version: &str) -> anyhow::Result<PathBuf> {
    let installer_url  = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforged_version}/neoforge-{neoforged_version}-installer.jar"
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
        return Err(anyhow::Error::msg("Neoforged website return error"));
    }

    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}
