// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Minecraft + loader installer.
//!
//! Owns the version-list requests, the install pipeline, the mod loader
//! installers, the Mojang Java runtime download and the first-launch language
//! setup. The version-list caches are process-wide statics.
//!
//! [`install`] is spawned by the app, which hands in the [`InstallSink`] its
//! progress is reported through and aborts the task to cancel. The crate owns
//! the sampling of its own [`DownloadState`], so the caller never sees a shared
//! counter.

// TODO: Support Optifine auto install

use std::{path::Path, str::FromStr, sync::Arc};

use log::{debug, info, warn};
use once_cell::sync::Lazy;
use quilt::QuiltVersionList;
use vanilla::generate_download_info;

use config::{Config, get_system_language};
use download::progress::{DownloadSnapshot, DownloadState};
use download::{Checksum, download_concurrent};
use instance::{Instance, InstanceRuntime, ModLoaderType};
use shared::HTTP_CLIENT;
use shared::{ChangeReporter, Sink};
use storage::LOCATIONS;
use version::{Version, resolve_version};

use crate::{forge::ForgeVersionList, vanilla::VersionManifest};

pub mod authlib_injector;
mod error;
pub mod fabric;
pub mod forge;
pub mod java;
pub mod language;
pub mod neoforge;
pub mod quilt;
pub mod vanilla;

pub use error::*;

/// How long a fetched version list stays fresh, in seconds.
static CACHE_EXPIRATION_SECONDS: u64 = 1800;

/// Seconds since the epoch, used as the cache timestamp.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The cached copy of a version list, if the last fetch is still fresh.
fn cached<T: Clone>(cache: &std::sync::Mutex<Option<(u64, T)>>) -> Option<T> {
    let guard = cache.lock().expect("Internal error");
    let (fetched_at, value) = guard.as_ref()?;
    (unix_now().saturating_sub(*fetched_at) < CACHE_EXPIRATION_SECONDS).then(|| value.clone())
}

/// Stores a freshly fetched list and hands it back.
fn store<T: Clone>(cache: &std::sync::Mutex<Option<(u64, T)>>, value: T) -> T {
    *cache.lock().expect("Internal error") = Some((unix_now(), value.clone()));
    value
}

static MANIFEST_CACHE: Lazy<std::sync::Mutex<Option<(u64, VersionManifest)>>> =
    Lazy::new(|| std::sync::Mutex::new(None));
static FORGE_VERSION_LIST_CACHE: Lazy<std::sync::Mutex<Option<(u64, ForgeVersionList)>>> =
    Lazy::new(|| std::sync::Mutex::new(None));
#[allow(clippy::type_complexity)]
static NEOFORGE_VERSION_LIST_CACHE: Lazy<std::sync::Mutex<Option<(u64, Vec<String>)>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

/// Every Minecraft version Mojang knows, newest first.
pub async fn get_minecraft_version_list() -> Result<VersionManifest> {
    if let Some(cached) = cached(&MANIFEST_CACHE) {
        return Ok(cached);
    }
    Ok(store(&MANIFEST_CACHE, VersionManifest::new().await?))
}

/// The Fabric loader versions for a Minecraft version. Not cached: the answer
/// depends on the Minecraft version.
pub async fn get_fabric_version_list(mcversion: &str) -> Result<fabric::LoaderArtifactList> {
    fabric::LoaderArtifactList::new(mcversion).await
}

/// The Quilt loader versions for a Minecraft version. Not cached, like Fabric.
pub async fn get_quilt_version_list(mcversion: &str) -> Result<QuiltVersionList> {
    QuiltVersionList::new(mcversion).await
}

/// Every Forge version, keyed by Minecraft version.
pub async fn get_forge_version_list() -> Result<ForgeVersionList> {
    if let Some(cached) = cached(&FORGE_VERSION_LIST_CACHE) {
        return Ok(cached);
    }
    Ok(store(
        &FORGE_VERSION_LIST_CACHE,
        ForgeVersionList::new().await?,
    ))
}

/// Every Neoforge version, newest first.
pub async fn get_neoforge_version_list() -> Result<Vec<String>> {
    if let Some(cached) = cached(&NEOFORGE_VERSION_LIST_CACHE) {
        return Ok(cached);
    }
    Ok(store(
        &NEOFORGE_VERSION_LIST_CACHE,
        neoforge::get_neoforge_version_list().await?,
    ))
}

/// What an install reports, in the order it reports it.
///
/// Each variant holds a plain [`DownloadSnapshot`] rather than a
/// [`DownloadState`], so an event can cross a thread and compare by value —
/// which is what lets the reporter drop the ticks where nothing moved.
#[derive(Debug, Clone, PartialEq)]
pub enum InstallProgress {
    Prepare,
    InstallGame(DownloadSnapshot),
    InstallJava(DownloadSnapshot),
    InstallModLoader(ModLoaderProgress),
}

/// Fine-grained progress of a mod loader installation.
#[derive(Debug, Clone, PartialEq)]
pub enum ModLoaderProgress {
    /// Preparing the installation (resolving versions and Java).
    Prepare,
    /// Downloading the loader installer JAR.
    DownloadInstaller(DownloadSnapshot),
    /// Prefetching the libraries named in the installer JAR (Forge).
    PrefetchDependencies(DownloadSnapshot),
    /// Running the installer subprocess, carrying its latest log line.
    RunInstaller { message: String },
}

/// The output port [`install`] reports through.
pub type InstallSink = Sink<InstallProgress>;

/// Reports [`ModLoaderProgress`] to the install's port.
///
/// The loader installers run on the install task but only know their own
/// sub-progress, so they report through this rather than the whole event enum.
#[derive(Clone)]
pub struct ModLoaderReporter {
    reporter: Arc<ChangeReporter<InstallProgress>>,
}

impl ModLoaderReporter {
    fn new(reporter: Arc<ChangeReporter<InstallProgress>>) -> Self {
        Self { reporter }
    }

    pub fn report(&self, progress: ModLoaderProgress) {
        self.reporter
            .report(InstallProgress::InstallModLoader(progress));
    }

    /// Reports one output line of the installer subprocess as
    /// [`ModLoaderProgress::RunInstaller`]. Empty lines are skipped and overly
    /// long lines are clamped to keep the payload small.
    pub fn report_installer_line(&self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.report(ModLoaderProgress::RunInstaller {
            message: line.chars().take(200).collect(),
        });
    }
}

/// Fetches the standard Maven `.sha1` checksum for the artifact behind `url`,
/// falling back to no checksum when it is unavailable.
pub(crate) async fn fetch_maven_sha1(url: &str) -> Checksum {
    let response = match HTTP_CLIENT.get(format!("{url}.sha1")).send().await {
        Ok(response) => response,
        Err(_) => return Checksum::None,
    };
    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(_) => return Checksum::None,
    };
    match response.text().await {
        Ok(text) => Checksum::Sha1(text.trim().to_string()),
        Err(_) => Checksum::None,
    }
}

/// Installs Minecraft, Java, and optionally a mod loader for the given instance.
///
/// Runs the full pipeline: download the game files, install Java, then install
/// the mod loader (Fabric, Forge, Quilt or NeoForge).
///
/// The caller spawns this and hands in the [`InstallSink`] the progress is
/// reported through, then aborts the task to cancel. The crate samples its own
/// download counters, so a caller sees only [`InstallProgress`] snapshots.
pub async fn install(config: Config, instance: Instance, sink: InstallSink) -> Result<()> {
    let reporter = Arc::new(ChangeReporter::new(sink));
    reporter.report(InstallProgress::Prepare);
    info!(
        "Start installing the game for instance {}",
        instance.config.name
    );
    let runtime = &instance.config.runtime;

    print_runtime_info(runtime);

    info!("Generating download task...");
    let download_list =
        generate_download_info(&runtime.minecraft, LOCATIONS.minecraft.clone()).await?;

    let progress = DownloadState::default();
    reporter.report(InstallProgress::InstallGame(progress.snapshot()));
    info!("Downloading files");
    download::progress::watch(
        &progress,
        |snapshot| reporter.report(InstallProgress::InstallGame(snapshot)),
        download_concurrent(download_list, &progress, config.download.clone()),
    )
    .await?;

    info!("Installing Java");
    let progress = DownloadState::default();
    reporter.report(InstallProgress::InstallJava(progress.snapshot()));

    if instance.config.launch_config.java_path.is_none() && config.prefer_mojang_java {
        download::progress::watch(
            &progress,
            |snapshot| reporter.report(InstallProgress::InstallJava(snapshot)),
            java::install_for_instance(&instance, &progress, config.download.clone()),
        )
        .await?;
    }

    if runtime.mod_loader_type.is_some() {
        info!("Install mod loader");
        reporter.report(InstallProgress::InstallModLoader(
            ModLoaderProgress::Prepare,
        ));
        let mod_reporter = ModLoaderReporter::new(Arc::clone(&reporter));
        let installer_java = resolve_installer_java(&config, &instance).await?;
        install_mod_loader(runtime, installer_java.path.as_path(), &mod_reporter).await?;
    };

    configure_first_launch_language(config, &instance).await;

    debug!("Saving lock file");
    tokio::fs::write(
        LOCATIONS
            .instances
            .get_instance_root(&instance.id)
            .join(".install.lock"),
        b"ok",
    )
    .await?;
    Ok(())
}

fn print_runtime_info(runtime: &InstanceRuntime) {
    info!("------------- Instance runtime config -------------");
    info!("-> Minecraft: {}", runtime.minecraft);
    match &runtime.mod_loader_type {
        Some(mod_loader_version) => info!("-> Mod loader: {mod_loader_version}"),
        None => info!("-> Mod loader: none"),
    };
    match &runtime.mod_loader_version {
        Some(mod_loader_version) => info!("-> Mod loader version: {mod_loader_version}"),
        None => info!("-> Mod loader version: none"),
    };
}

/// Resolves the Java executable used to run mod loader installers, applying the
/// same selection logic as game launching (see `java_discovery::resolve_java_executable`).
async fn resolve_installer_java(
    config: &Config,
    instance: &Instance,
) -> Result<java_discovery::ResolvedJava> {
    let minecraft_location = LOCATIONS.minecraft.clone();
    let version_json_path = minecraft_location.get_version_json(&instance.config.runtime.minecraft);
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await?;
    let unresolved_version = serde_json::from_str::<Version>(&raw_version_json)?;
    let resolved_version = resolve_version(&unresolved_version, &minecraft_location, &[])?;
    Ok(
        java_discovery::resolve_java_executable(&java_discovery::ResolveJavaOptions {
            instance_java_path: instance.config.launch_config.java_path.clone(),
            prefer_mojang_java: config.prefer_mojang_java,
            disabled_java_runtimes: config.disabled_java_runtime.clone(),
            required_major_version: resolved_version.java_version.major_version as u32,
            mojang_component: resolved_version.java_version.component.clone(),
        })
        .await?,
    )
}

/// Installs the mod loader named by the runtime configuration.
///
/// `java_path` is only used by the Java-based installers (Forge and NeoForge).
/// Fails if the loader type or version is missing, or if the installer fails.
pub async fn install_mod_loader(
    runtime: &InstanceRuntime,
    java_path: &Path,
    reporter: &ModLoaderReporter,
) -> Result<()> {
    let mod_loader_type = runtime
        .mod_loader_type
        .as_ref()
        .ok_or(Error::InstanceBroken)?;
    let mod_loader_version = runtime
        .mod_loader_version
        .as_ref()
        .ok_or(Error::InstanceBroken)?;
    match mod_loader_type {
        ModLoaderType::Fabric => {
            fabric::install(
                &runtime.minecraft,
                mod_loader_version,
                LOCATIONS.minecraft.clone(),
            )
            .await?
        }
        ModLoaderType::Quilt => {
            quilt::install(
                &runtime.minecraft,
                mod_loader_version,
                LOCATIONS.minecraft.clone(),
            )
            .await?
        }
        ModLoaderType::Forge => {
            forge::install(
                &LOCATIONS.minecraft,
                mod_loader_version,
                &runtime.minecraft,
                java_path,
                reporter,
            )
            .await?
        }
        ModLoaderType::Neoforge => {
            neoforge::install(
                &LOCATIONS.minecraft.root,
                mod_loader_version,
                java_path,
                reporter,
            )
            .await?
        }
    }

    Ok(())
}

/// Sets the in-game language to match the launcher UI language before the first
/// launch, by writing `options.txt` in the instance game directory.
///
/// Only applies when the "change game language on first launch" setting is
/// enabled. A failure to write the file is logged and does not abort the
/// installation.
async fn configure_first_launch_language(config: Config, instance: &Instance) {
    if !config.accessibility.change_game_language {
        return;
    }
    let options_txt_path = LOCATIONS
        .instances
        .get_instance_root(&instance.id)
        .join("options.txt");
    let launcher_language = config
        .language
        .as_deref()
        .unwrap_or_else(|| get_system_language());
    let release_time = get_version_release_time(instance).await;
    if let Err(error) = language::configure_game_language(
        &options_txt_path,
        launcher_language,
        release_time.as_deref(),
    )
    .await
    {
        warn!("Failed to configure the game language: {error}");
    }
}

/// Reads the `releaseTime` of the Minecraft version from the version.json that has
/// already been downloaded during the installation.
async fn get_version_release_time(instance: &Instance) -> Option<String> {
    let minecraft_location = LOCATIONS.minecraft.clone();
    let version_json_path = minecraft_location.get_version_json(&instance.config.runtime.minecraft);
    // Both of these used to be a silent `None`, and the consequence is not a
    // quiet value: `language_era(None)` answers `Modern`, so an unreadable
    // `version.json` gives a pre-1.6 instance the modern `lang` casing and the
    // game silently falls back to English for want of the right resource pack.
    let raw_version_json = match tokio::fs::read_to_string(&version_json_path).await {
        Ok(raw) => raw,
        Err(error) => {
            warn!(
                "Could not read {} to decide the game's language era: {error}",
                version_json_path.display()
            );
            return None;
        }
    };
    match Version::from_str(&raw_version_json) {
        Ok(version) => version.release_time,
        Err(error) => {
            warn!(
                "{} does not parse, so the game's language era is unknown: {error}",
                version_json_path.display()
            );
            None
        }
    }
}
