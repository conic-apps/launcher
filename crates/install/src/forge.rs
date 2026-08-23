// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The `forge` module contains functionality related to Forge installation and version management.
//!
//! This module re-exports the `install` function from the `install` submodule,
//! and exposes the `version_list` submodule for managing Forge versions.

use std::{
    cmp::Reverse,
    collections::HashMap,
    ffi::OsString,
    io::BufRead,
    path::{Path, PathBuf},
    process::{Child, Stdio},
};

use config::download::DownloadConfig;
use download::{DownloadTask, DownloadTaskType, download_concurrent, progress::DownloadState};
use folder::{DATA_LOCATION, MinecraftLocation};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;

use platform::DELIMITER;
use version::{Version, resolve_libraries};
use zip::ZipArchive;

use crate::{
    ModLoaderProgress, ModLoaderReporter, error::*, vanilla::generate_libraries_downloads,
};

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
        let mut list: Self = HTTP_CLIENT
            .get("https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json")
            .send()
            .await?
            .json::<Self>()
            .await?;
        for versions in list.0.values_mut() {
            versions.sort_by_cached_key(|version| Reverse(tokenize_version(version)));
        }
        Ok(list)
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

/// Splits a Forge version string into [`VersionToken`]s.
///
/// `.` and `-` act as separators; digit runs become [`VersionToken::Num`],
/// all other runs become [`VersionToken::Text`] (branch suffixes such as
/// `-1.7.10` or `-prerelease`).
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

/// Forge Install Bootstrapper - by bangbang93
/// [GitHub Repository](https://github.com/bangbang93/forge-install-bootstrapper)
///
/// Embedded JAR file used for bootstrapping Forge installation on newer Forge versions.
const FORGE_INSTALL_BOOTSTRAPPER_BANGBANG93: &[u8] =
    include_bytes!("./forge-install-bootstrapper(bangbang93).jar");
/// Forge Install Bootstrapper - by ConicMC
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
/// * `minecraft_location` - The directory where Forge should be installed.
/// * `forge_version` - The Forge version string to install (e.g., "1.20.1-47.1.0").
/// * `mcversion` - The Minecraft version string associated with this Forge version.
/// * `java_path` - The Java executable used to run the installer.
/// * `reporter` - Progress reporter forwarded to the frontend.
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
    java_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    info!("Start downloading the forge installer");
    let installer_path = download_installer(mcversion, forge_version, reporter).await?;
    let _ = prefetch_installer_dependencies(minecraft_location, &installer_path, reporter).await;
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
/// * `reporter` - Progress reporter forwarded to the frontend.
///
/// # Returns
///
/// A `PathBuf` pointing to the downloaded installer JAR.
///
/// # Errors
///
/// Returns an error if the download fails or the file cannot be written.
pub async fn download_installer(
    mcversion: &str,
    forge_version: &str,
    reporter: &ModLoaderReporter,
) -> Result<PathBuf> {
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
        reporter.report(ModLoaderProgress::PrefetchDependencies(progress.clone()));
        download_concurrent(download_entries, &progress, DownloadConfig::default()).await?;
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
    let child = std::process::Command::new(java_path)
        .arg("-cp")
        .arg(generate_classpath(
            &bangbang93_bootstrapper_path,
            installer_path,
        )?)
        .arg("com.bangbang93.ForgeInstaller")
        .arg(install_dir)
        .stdout(Stdio::piped())
        .spawn()?;
    let result = wait_child(child, reporter);
    async_fs::remove_file(bangbang93_bootstrapper_path).await?;
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
    let child = std::process::Command::new(java_path)
        .arg("-cp")
        .arg(generate_classpath(
            &conicmc_bootstrapper_path,
            installer_path,
        )?)
        .arg("app.conicmc.Bootstrap")
        .arg(install_dir)
        .arg(version_id)
        .stdout(Stdio::piped())
        .spawn()?;
    let result = wait_child(child, reporter);
    async_fs::remove_file(conicmc_bootstrapper_path).await?;
    result
}

fn wait_child(mut child: Child, reporter: &ModLoaderReporter) -> Result<()> {
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
        } else {
            debug!("[{pid}] {line}");
            reporter.report_installer_line(line);
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
