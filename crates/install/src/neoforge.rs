// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Neoforge version list and installer.

use std::{path::Path, path::PathBuf, process::Stdio};

use log::{debug, error, info};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use config::download::DownloadConfig;
use download::{DownloadTask, DownloadTaskType, download_concurrent, progress::DownloadState};
use shared::HTTP_CLIENT;
use storage::LOCATIONS;

use crate::{ModLoaderProgress, ModLoaderReporter, error::*};

/// Fetches every published Neoforge version, newest first.
///
/// Modern versions live under `net/neoforged/neoforge`, the pre-1.20.2
/// ("legacy") ones under `net/neoforged/forge`; both are appended into one
/// list.
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

/// Downloads and runs the Neoforge installer, then removes the temporary JAR.
pub async fn install(
    install_dir: &PathBuf,
    neoforge_version: &str,
    java_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    info!("Start downloading the neoforge installer");
    let installer_path = download_installer(neoforge_version, reporter).await?;
    info!("Running installer with {}", java_path.display());

    let mut child = Command::new(java_path)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()?;

    let out = child.stdout.take().ok_or(Error::NeoforgeInstallerFailed)?;
    let mut out = BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    let pid = child.id().ok_or(Error::NeoforgeInstallerFailed)?;

    loop {
        buf.clear();
        let size = out.read_line(&mut buf).await?;
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

    let status = child.wait().await?;
    tokio::fs::remove_file(installer_path).await?;
    if !success || !status.success() {
        error!("Failed to ran neoforge installer");
        return Err(Error::NeoforgeInstallerFailed);
    }
    Ok(())
}

/// Downloads the Neoforge installer JAR to a temp file and returns its path.
pub async fn download_installer(
    neoforge_version: &str,
    reporter: &ModLoaderReporter,
) -> Result<PathBuf> {
    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar"
    );
    info!("The installer url is: {installer_url}");

    let installer_path = LOCATIONS
        .launcher
        .temp
        .join(format!("{}.jar", uuid::Uuid::new_v4()));
    if let Some(parent) = installer_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let checksum = crate::fetch_maven_sha1(&installer_url).await;
    let progress = DownloadState::default();
    reporter.report(ModLoaderProgress::DownloadInstaller(progress.snapshot()));
    download::progress::watch(
        &progress,
        |snapshot| reporter.report(ModLoaderProgress::DownloadInstaller(snapshot)),
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
        ),
    )
    .await?;
    Ok(installer_path)
}
