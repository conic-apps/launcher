// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::BufRead,
    path::PathBuf,
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};

use folder::DATA_LOCATION;
use futures::AsyncWriteExt;
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;

use crate::error::*;

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
    /// * `Err(Error)` if the request fails or the data cannot be parsed.
    pub async fn new() -> Result<Self> {
        Ok(HTTP_CLIENT
            .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge")
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn from_mcversion(mcversion: &str) -> Result<Vec<String>> {
        let version_list = NeoforgedVersionList::new().await?;
        let splited_mcversion: Vec<&str> = mcversion.split('.').collect();
        Ok(version_list
            .versions
            .into_iter()
            .rev()
            .filter(|x| {
                let splited_version: Vec<&str> = x.split('.').collect();
                #[allow(clippy::get_first)]
                return splited_version.get(0) == splited_mcversion.get(1)
                    && (splited_version.get(1) == splited_mcversion.get(2)
                        || (splited_version.get(1) == Some(&"0")
                            && splited_mcversion.get(2).is_none()));
            })
            .collect::<Vec<String>>())
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
/// * `Err(Error)` if installation fails.
pub async fn install(install_dir: &PathBuf, neoforged_version: &str) -> Result<()> {
    info!("Start downloading the neoforged installer");
    let installer_path = download_installer(neoforged_version).await?;
    let java = "/bin/java"; // TODO: Use config file
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
        .ok_or(Error::NeoforgedInstallerFailed)?;
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
                trace!("[{pid}] {last}");
            }
        }
    }

    let output = command.wait_with_output()?;
    if !success || !output.status.success() {
        async_fs::remove_file(installer_path).await?;
        error!("Failed to ran neoforged installer");
        return Err(Error::NeoforgedInstallerFailed);
    }

    async_fs::remove_file(installer_path).await?;
    Ok(())
}

/// Downloads the Neoforged installer JAR for the given version.
///
/// # Arguments
/// * `neoforged_version` - The version to download.
///
/// # Returns
/// * `Ok(PathBuf)` containing the path to the downloaded installer.
/// * `Err(Error)` if downloading fails.
async fn download_installer(neoforged_version: &str) -> Result<PathBuf> {
    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforged_version}/neoforge-{neoforged_version}-installer.jar"
    );
    info!("The installer url is: {installer_url}");

    let installer_path = DATA_LOCATION.temp.join(format!(
        "{}.jar",
        uuid::Uuid::from_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Incorrect system time")
                .as_nanos(),
        )
    ));
    if let Some(parent) = installer_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }

    let mut file = async_fs::File::create(&installer_path).await?;
    let response = HTTP_CLIENT.get(installer_url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::HttpResponseNotSuccess(
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown").to_string(),
        ));
    }
    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}
