// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Neoforge version list and installer.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
};

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
    info!(
        "Running {} -jar {} --installClient {}",
        java_path.display(),
        installer_path.display(),
        install_dir.display()
    );

    let mut child = Command::new(java_path)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(install_dir)
        .stdout(Stdio::piped())
        // Inherited by default, which sent NeoForge's own error output past the
        // launcher entirely; piped so it is reported like stdout.
        .stderr(Stdio::piped())
        .spawn()?;

    let out = child.stdout.take().ok_or(Error::NeoforgeInstallerFailed)?;
    let err = child.stderr.take();
    let pid = child.id().ok_or(Error::NeoforgeInstallerFailed)?;
    // Both streams are drained concurrently: reading one to EOF before touching
    // the other blocks as soon as the other's pipe buffer fills, which is a
    // few kilobytes of NeoForge error output.
    let success = Arc::new(AtomicBool::new(false));

    // The pumps are named futures rather than inline `async` blocks so each one's
    // `io::Result` is named rather than inferred from an ambiguous `?`.
    let stdout_pump = pump(Some(out), pid, reporter, Some(Arc::clone(&success)));
    let err_pump = pump(err, pid, reporter, None);

    let (stdout_result, stderr_result) = tokio::join!(stdout_pump, err_pump);
    stdout_result?;
    stderr_result?;

    let status = child.wait().await?;
    let success = success.load(AtomicOrdering::SeqCst);
    // The temp file is named by a bare UUID, so naming the version here is the
    // only thing that ties a leftover jar in the temp folder to an install.
    tokio::fs::remove_file(&installer_path)
        .await
        .map_err(|error| {
            // Deliberately not fatal: the install itself succeeded, and failing on a
            // cleanup turns a finished install into an error.
            log::warn!(
                "Could not remove the staged installer {}: {error}",
                installer_path.display()
            );
            error
        })?;
    if !success || !status.success() {
        error!(
            "Failed to run the neoforge installer: it {} and exited with {status}",
            if success {
                "reported success"
            } else {
                "never reported success"
            }
        );
        return Err(Error::NeoforgeInstallerFailed);
    }
    info!("neoforge {neoforge_version} installed");
    Ok(())
}

/// Reads `stream` to EOF, reporting every line.
///
/// `stream` is `None` when the caller did not pipe that stream. `success` is
/// `Some` only for the stdout stream, which is where NeoForge prints its
/// "Successfully installed client into launcher" handshake — a line matching that
/// on stderr is installer output, not the signal.
async fn pump<R>(
    stream: Option<R>,
    pid: u32,
    reporter: &ModLoaderReporter,
    success: Option<Arc<AtomicBool>>,
) -> std::result::Result<(), std::io::Error>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(stream) = stream else {
        return Ok(());
    };
    let mut reader = BufReader::new(stream);
    let mut buf = String::new();
    loop {
        buf.clear();
        if reader.read_line(&mut buf).await? == 0 {
            return Ok(());
        }
        let line = buf.trim().to_string();
        if success.is_some() && line.contains("Successfully installed client into launcher") {
            if let Some(success) = &success {
                success.store(true, AtomicOrdering::SeqCst);
            }
            info!("Successfully ran the neoforge installer");
        } else {
            debug!("[{pid}] {line}");
            reporter.report_installer_line(&line);
        }
    }
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
    info!("Downloaded the neoforge installer");
    Ok(installer_path)
}
