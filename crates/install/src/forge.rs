// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Forge version list and installer.

use std::{
    cmp::Reverse,
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, atomic::AtomicBool, atomic::Ordering as AtomicOrdering},
};

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

use config::download::DownloadConfig;
use download::{DownloadTask, DownloadTaskType, download_concurrent, progress::DownloadState};
use platform::{DELIMITER, strip_unc_prefix};
use shared::HTTP_CLIENT;
use storage::{LOCATIONS, MinecraftLocation};
use version::{Version, resolve_libraries};
use zip::ZipArchive;

use crate::{
    ModLoaderProgress, ModLoaderReporter, error::*, vanilla::generate_libraries_downloads,
};

/// A list of Forge versions for a given Minecraft version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionList(HashMap<String, Vec<String>>);

impl ForgeVersionList {
    /// Fetches every Forge version, keyed by Minecraft version.
    pub async fn new() -> Result<Self> {
        let mut list: Self = HTTP_CLIENT
            .get("https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json")
            .send()
            .await?
            .json()
            .await?;
        for versions in list.0.values_mut() {
            versions.sort_by_cached_key(|version| Reverse(tokenize_version(version)));
        }
        Ok(list)
    }

    /// The Forge versions of a Minecraft version, newest first.
    ///
    /// `None` when the list has no entry for that Minecraft version at all.
    pub fn get(&self, mcversion: &str) -> Option<&[String]> {
        self.0.get(mcversion).map(Vec::as_slice)
    }
}

/// A token of a Forge version string used for natural ordering.
///
/// Runs of digits compare numerically, everything else lexicographically,
/// so e.g. `36.0.10` correctly sorts after `36.0.9`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum VersionToken {
    Num(u64),
    Text(String),
}

/// `.` and `-` separate tokens, so a branch suffix like `-prerelease` stays
/// text rather than joining the preceding number.
fn tokenize_version(version: &str) -> Vec<VersionToken> {
    fn push(tokens: &mut Vec<VersionToken>, run: &mut String) {
        if run.is_empty() {
            return;
        }
        if run.chars().all(|c| c.is_ascii_digit()) {
            let num: u64 = run.parse().unwrap_or(u64::MAX);
            tokens.push(VersionToken::Num(num));
        } else {
            tokens.push(VersionToken::Text(run.clone()));
        }
        run.clear();
    }

    let mut tokens = Vec::new();
    let mut run = String::new();
    for c in version.chars() {
        match c {
            '.' | '-' => push(&mut tokens, &mut run),
            _ => run.push(c),
        }
    }
    push(&mut tokens, &mut run);
    tokens
}

/// Embedded bootstrapper JAR for installing newer Forge versions, from
/// [bangbang93/forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper).
const FORGE_INSTALL_BOOTSTRAPPER_BANGBANG93: &[u8] =
    include_bytes!("./forge-install-bootstrapper(bangbang93).jar");
/// Embedded bootstrapper JAR for legacy Forge installation, from
/// [conic-apps/forge-install-bootstrapper-legacy](https://github.com/conic-apps/forge-install-bootstrapper-legacy).
const FORGE_INSTALL_BOOTSTRAPPER_CONIC: &[u8] =
    include_bytes!("./forge-install-bootstrapper(conic).jar");

/// Installs the given Forge version into the target directory.
///
/// Tries the bundled bangbang93 bootstrapper, falling back to the ConicMC
/// bootstrapper for legacy versions.
pub async fn install(
    minecraft_location: &MinecraftLocation,
    forge_version: &str,
    mcversion: &str,
    java_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    info!("Start downloading the forge installer");
    let installer_path = download_installer(mcversion, forge_version, reporter).await?;
    // Prefetch is best-effort — the install can still proceed and re-download
    // later — but a failure must not be silent.
    if let Err(error) =
        prefetch_installer_dependencies(minecraft_location, &installer_path, reporter).await
    {
        warn!("Could not prefetch the forge installer dependencies: {error}");
    }
    let bangbang93_bootstrapper_installation_result = try_bangbang93_bootstrapper(
        &minecraft_location.root,
        &installer_path,
        java_path,
        reporter,
    )
    .await;
    // The legacy bootstrapper renames the installed version to this id, so it
    // must stay in sync with `Instance::get_version_id` (crates/instance).
    let version_id = format!("{mcversion}-forge-{forge_version}");
    let conicmc_bootstrapper_installation_result =
        if let Err(Error::ForgeInstallerFailed) = bangbang93_bootstrapper_installation_result {
            Some(
                try_conicmc_bootstrapper(
                    &minecraft_location.root,
                    &installer_path,
                    java_path,
                    &version_id,
                    reporter,
                )
                .await,
            )
        } else {
            None
        };
    tokio::fs::remove_file(installer_path).await?;
    merge_results(
        bangbang93_bootstrapper_installation_result,
        conicmc_bootstrapper_installation_result,
    )
}

/// Downloads the Forge installer JAR to a temporary file and returns its path.
pub async fn download_installer(
    mcversion: &str,
    forge_version: &str,
    reporter: &ModLoaderReporter,
) -> Result<PathBuf> {
    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{mcversion}-{forge_version}/forge-{mcversion}-{forge_version}-installer.jar"
    );
    info!("The installer url is: {installer_url}");
    let installer_path = LOCATIONS
        .launcher
        .temp
        .join(format!("forge-installer-{forge_version}.jar"));
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
    info!("Downloaded forge installer");
    Ok(installer_path)
}

async fn prefetch_installer_dependencies(
    minecraft_location: &MinecraftLocation,
    installer_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    let file = std::fs::File::open(installer_path)?;
    let mut archive = ZipArchive::new(file)?;
    let version_file = archive.by_name("version.json")?;
    let version: Version = serde_json::from_reader(version_file)?;
    if let Some(libraries) = version.libraries {
        let libraries = resolve_libraries(libraries)?;
        let download_entries = generate_libraries_downloads(minecraft_location, &libraries);
        let progress = DownloadState::default();
        reporter.report(ModLoaderProgress::PrefetchDependencies(progress.snapshot()));
        download::progress::watch(
            &progress,
            |snapshot| reporter.report(ModLoaderProgress::PrefetchDependencies(snapshot)),
            download_concurrent(download_entries, &progress, DownloadConfig::default()),
        )
        .await?;
    }
    Ok(())
}

async fn try_bangbang93_bootstrapper(
    install_dir: &Path,
    installer_path: &Path,
    java_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    info!("Trying Bangbang93 forge install bootstrapper");
    let bangbang93_bootstrapper_path =
        save_bootstrapper(FORGE_INSTALL_BOOTSTRAPPER_BANGBANG93).await?;
    // Log the java used: a spawn failure would otherwise name neither the
    // runtime nor the classpath.
    info!(
        "Running {} with the bangbang93 bootstrapper",
        java_path.display()
    );
    let child = Command::new(java_path)
        .arg("-cp")
        .arg(generate_classpath(
            &bangbang93_bootstrapper_path,
            installer_path,
        )?)
        .arg("com.bangbang93.ForgeInstaller")
        .arg(install_dir)
        .stdout(Stdio::piped())
        // A Gradle stack trace on stderr is the most informative thing the
        // installer produces, so it is piped and reported rather than inherited.
        .stderr(Stdio::piped())
        .spawn()?;
    let result = wait_child(child, reporter).await;
    tokio::fs::remove_file(bangbang93_bootstrapper_path).await?;
    result
}

/// Installs legacy Forge using the ConicMC bootstrapper.
///
/// The bootstrapper rewrites the installer's embedded `install_profile.json`
/// so the installed version directory is named `version_id` instead of the
/// era-specific name chosen by the official installer.
async fn try_conicmc_bootstrapper(
    install_dir: &Path,
    installer_path: &Path,
    java_path: &Path,
    version_id: &str,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    info!("Trying ConicMC forge install bootstrapper");
    let conicmc_bootstrapper_path = save_bootstrapper(FORGE_INSTALL_BOOTSTRAPPER_CONIC).await?;
    info!(
        "Running {} with the ConicMC bootstrapper",
        java_path.display()
    );
    let child = Command::new(java_path)
        .arg("-cp")
        .arg(generate_classpath(
            &conicmc_bootstrapper_path,
            installer_path,
        )?)
        .arg("app.conicmc.Bootstrap")
        .arg(install_dir)
        .arg(version_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let result = wait_child(child, reporter).await;
    tokio::fs::remove_file(conicmc_bootstrapper_path).await?;
    result
}

/// Streams the installer's stdout and stderr, reporting every line but the
/// `true` handshake, then waits for the process to exit.
///
/// Driven by the async [`Command`] because the install can run for minutes; a
/// blocking `read_line` would park a runtime worker for the whole install.
///
/// Both streams are drained concurrently: draining one to EOF first blocks once
/// the other pipe fills, and a failing installer usually explains itself on
/// stderr.
async fn wait_child(mut child: Child, reporter: &ModLoaderReporter) -> Result<()> {
    let stdout = child.stdout.take().ok_or(Error::ForgeInstallerFailed)?;
    let stderr = child.stderr.take();
    let pid = child.id().ok_or(Error::ForgeInstallerFailed)?;
    // Shared because each stream is pumped on its own future and they have to
    // agree on one verdict: only stdout carries the `true` handshake.
    let success = Arc::new(AtomicBool::new(false));

    let stdout_pump = pump_lines(stdout, pid, reporter, Some(Arc::clone(&success)));
    // stderr is optional: a caller that did not pipe it hands back `None`.
    let stderr_pump = async {
        match stderr {
            Some(stderr) => pump_lines(stderr, pid, reporter, None).await,
            None => Ok(()),
        }
    };

    let (stdout_result, stderr_result) = tokio::join!(stdout_pump, stderr_pump);
    stdout_result?;
    stderr_result?;

    let status = child.wait().await?;
    // Distinguish "printed `true` and then exited non-zero" from "never printed
    // it": the status carries the exit code and signal that separate the two.
    let reported_success = success.load(AtomicOrdering::SeqCst);
    if !reported_success || !status.success() {
        error!(
            "Failed to run forge installer: it {} and exited with {status}",
            if reported_success {
                "reported success"
            } else {
                "never reported success"
            }
        );
        return Err(Error::ForgeInstallerFailed);
    }
    Ok(())
}

/// Reads `stream` to EOF, reporting every line.
///
/// `success` is `Some` only for the stream carrying the `true` handshake — a
/// `true` on stderr is installer output, not the handshake.
async fn pump_lines<R>(
    stream: R,
    pid: u32,
    reporter: &ModLoaderReporter,
    success: Option<Arc<AtomicBool>>,
) -> std::result::Result<(), std::io::Error>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut buf = String::new();
    loop {
        buf.clear();
        if reader.read_line(&mut buf).await? == 0 {
            return Ok(());
        }
        let line = buf.trim().to_string();
        if success.is_some() && line == "true" {
            if let Some(success) = &success {
                success.store(true, AtomicOrdering::SeqCst);
            }
            info!("Successfully ran the forge installer");
        } else {
            debug!("[{pid}] {line}");
            reporter.report_installer_line(&line);
        }
    }
}

async fn save_bootstrapper(data: &[u8]) -> Result<PathBuf> {
    let bootstrapper_path = LOCATIONS
        .launcher
        .temp
        .join("forge-install-bootstrapper.jar");
    tokio::fs::write(&bootstrapper_path, data).await?;
    Ok(bootstrapper_path)
}

fn generate_classpath(bootstrapper_path: &Path, installer_path: &Path) -> Result<OsString> {
    let bootstrapper = strip_unc_prefix(bootstrapper_path.canonicalize()?);
    let installer = strip_unc_prefix(installer_path.canonicalize()?);
    let mut result = bootstrapper.into_os_string();
    result.push(DELIMITER);
    result.push(installer.into_os_string());
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
