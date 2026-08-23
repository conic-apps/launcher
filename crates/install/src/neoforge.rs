// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{io::BufRead, path::Path, path::PathBuf, process::Stdio};

use config::download::DownloadConfig;
use download::{DownloadTask, DownloadTaskType, download_concurrent, progress::DownloadState};
use folder::DATA_LOCATION;
use log::{debug, error, info};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::{ModLoaderProgress, ModLoaderReporter, error::*};

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
/// Downloads the installer, runs it using the given Java Runtime,
/// and then cleans up the temporary installer file.
///
/// # Arguments
/// * `install_dir` - The target directory where the client will be installed.
/// * `neoforge_version` - The version of Neoforge to install.
/// * `java_path` - The Java executable used to run the installer.
/// * `reporter` - Progress reporter forwarded to the frontend.
///
/// # Returns
/// * `Ok(())` on successful installation.
/// * `Err(Error)` if installation fails.
pub async fn install(
    install_dir: &PathBuf,
    neoforge_version: &str,
    java_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    info!("Start downloading the neoforge installer");
    let installer_path = download_installer(neoforge_version, reporter).await?;
    info!("Running installer with {}", java_path.display());

    let mut command = std::process::Command::new(java_path)
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
        if line.contains("Successfully installed client into launcher") {
            success = true;
            info!("Successfully ran the neoforge installer");
        } else {
            debug!("[{pid}] {line}");
            reporter.report_installer_line(line);
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
///
/// * `neoforge_version` - The version to download.
/// * `reporter` - Progress reporter forwarded to the frontend.
///
/// # Returns
///
/// * `Ok(PathBuf)` containing the path to the downloaded installer.
/// * `Err(Error)` if downloading fails.
pub async fn download_installer(
    neoforge_version: &str,
    reporter: &ModLoaderReporter,
) -> Result<PathBuf> {
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

    let checksum = crate::fetch_maven_sha1(&installer_url).await;
    let progress = DownloadState::default();
    reporter.report(ModLoaderProgress::DownloadInstaller(progress.clone()));
    download_concurrent(
        vec![DownloadTask {
            url: installer_url,
            file: installer_path.clone(),
            checksum,
            size_bytes: None,
            task_type: DownloadTaskType::Unknown,
        }],
        &progress,
        DownloadConfig::default(),
    )
    .await?;
    Ok(installer_path)
}
