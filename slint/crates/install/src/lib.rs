// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/install`: the Minecraft + loader installer.
//!
//! The original crate exposes its work through Tauri commands that own the
//! plugin state and forward progress over an IPC `Channel`. This mirror keeps
//! the whole domain layer — the version-list requests, the install pipeline,
//! the mod loader installers, the Mojang Java runtime download and the
//! first-launch language setup — and drops only the command layer:
//!
//!   * the caches that lived in the Tauri `PluginState` are statics here;
//!   * [`install`] is the body of the original `cmd_spawn_install_task`, taking
//!     the same `Arc<Mutex<InstallEvent>>` the command thread used to poll. The
//!     app spawns it on its own runtime, keeps the `JoinHandle` for
//!     cancellation and translates the polled events into UI state, playing the
//!     role of `cmd_spawn_install_task` + the channel.
//!
//! Structures, request URLs and sorting match `crates/install/src/*.rs` so the
//! two crates can be diffed against each other.

// TODO: Support Optifine auto install

use std::{path::Path, str::FromStr, sync::Arc};

use log::{debug, info, warn};
use once_cell::sync::Lazy;
use quilt::QuiltVersionList;
use serde::Serialize;
use vanilla::generate_download_info;

use slint_config::{Config, get_system_language};
use slint_download::progress::DownloadState;
use slint_download::{Checksum, download_concurrent};
use slint_folder::{DATA_LOCATION, MinecraftLocation};
use slint_instance::{Instance, InstanceRuntime, ModLoaderType};
use slint_shared::HTTP_CLIENT;
use slint_version::{Version, resolve_version};

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

/// How long a fetched version list stays fresh, mirroring
/// `CACHE_EXPIRATION_SECONDS` in `crates/install/src/lib.rs`.
static CACHE_EXPIRATION_SECONDS: u64 = 1800;

/// Seconds since the epoch, used as the cache timestamp.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The cached copy of a version list, if the last fetch is still fresh.
///
/// `crates/install` keeps these in its Tauri `PluginState`; the freshness test
/// here is the intended one (the original compares the age the other way round,
/// so it only ever serves a copy that is *older* than the TTL).
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
// The original carries the same allowance on the field of its `PluginState`.
#[allow(clippy::type_complexity)]
static NEOFORGE_VERSION_LIST_CACHE: Lazy<std::sync::Mutex<Option<(u64, Vec<String>)>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

/// Every Minecraft version Mojang knows, newest first (`cmd_get_minecraft_version_list`).
pub async fn get_minecraft_version_list() -> Result<VersionManifest> {
    if let Some(cached) = cached(&MANIFEST_CACHE) {
        return Ok(cached);
    }
    Ok(store(&MANIFEST_CACHE, VersionManifest::new().await?))
}

/// The Fabric loader versions for a Minecraft version
/// (`cmd_get_fabric_version_list`). Not cached: the answer depends on the
/// Minecraft version and the original caches it per plugin instance only.
pub async fn get_fabric_version_list(mcversion: &str) -> Result<fabric::LoaderArtifactList> {
    fabric::LoaderArtifactList::new(mcversion).await
}

/// The Quilt loader versions for a Minecraft version
/// (`cmd_get_quilt_version_list`). Not cached, like Fabric.
pub async fn get_quilt_version_list(mcversion: &str) -> Result<QuiltVersionList> {
    QuiltVersionList::new(mcversion).await
}

/// Every Forge version, keyed by Minecraft version
/// (`cmd_get_forge_version_list`).
pub async fn get_forge_version_list() -> Result<ForgeVersionList> {
    if let Some(cached) = cached(&FORGE_VERSION_LIST_CACHE) {
        return Ok(cached);
    }
    Ok(store(
        &FORGE_VERSION_LIST_CACHE,
        ForgeVersionList::new().await?,
    ))
}

/// Every Neoforge version, newest first (`cmd_get_neoforge_version_list`).
pub async fn get_neoforge_version_list() -> Result<Vec<String>> {
    if let Some(cached) = cached(&NEOFORGE_VERSION_LIST_CACHE) {
        return Ok(cached);
    }
    Ok(store(
        &NEOFORGE_VERSION_LIST_CACHE,
        neoforge::get_neoforge_version_list().await?,
    ))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "job", content = "progress")]
pub enum InstallEvent {
    Prepare,
    InstallGame(DownloadState),
    InstallJava(DownloadState),
    InstallModLoader(ModLoaderProgress),
}

/// Fine-grained progress of a mod loader installation.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "phase", content = "detail", rename_all = "camelCase")]
pub enum ModLoaderProgress {
    /// Preparing the installation (resolving versions and Java).
    Prepare,
    /// Downloading the loader installer JAR.
    DownloadInstaller(DownloadState),
    /// Prefetching libraries bundled inside the installer JAR (Forge).
    PrefetchDependencies(DownloadState),
    /// Running the installer subprocess, carrying its latest log line.
    RunInstaller { message: String },
}

/// Clonable handle through which loader installers report [`ModLoaderProgress`].
#[derive(Clone)]
pub struct ModLoaderReporter {
    status: Arc<std::sync::Mutex<InstallEvent>>,
}

impl ModLoaderReporter {
    pub(crate) fn new(status: &Arc<std::sync::Mutex<InstallEvent>>) -> Self {
        Self {
            status: Arc::clone(status),
        }
    }

    pub fn report(&self, progress: ModLoaderProgress) {
        let mut current = self.status.lock().expect("Internal error");
        *current = InstallEvent::InstallModLoader(progress);
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
/// This function runs a full installation pipeline including:
/// - Downloading Minecraft game files
/// - Installing Java
/// - Installing a mod loader (Fabric, Forge, Quilt, NeoForge)
///
/// The body is the original `cmd_spawn_install_task`'s, minus the Tauri
/// `PluginState` ownership: the caller spawns it, hands in the shared status it
/// polls, and aborts the task to cancel.
pub async fn install(
    config: Config,
    instance: Instance,
    status: Arc<std::sync::Mutex<InstallEvent>>,
) -> Result<()> {
    {
        let mut status = status.lock().expect("Internal Error");
        *status = InstallEvent::Prepare;
    }
    info!(
        "Start installing the game for instance {}",
        instance.config.name
    );
    let runtime = &instance.config.runtime;

    print_runtime_info(runtime);

    info!("Generating download task...");
    let download_list = generate_download_info(
        &runtime.minecraft,
        MinecraftLocation::new(&DATA_LOCATION.root),
    )
    .await?;

    let progress = DownloadState::default();
    {
        let mut status = status.lock().expect("internal error");
        *status = InstallEvent::InstallGame(progress.clone())
    }
    info!("Downloading files");
    download_concurrent(download_list, &progress, config.download.clone()).await?;

    info!("Installing Java");
    let progress = DownloadState::default();
    {
        let mut status = status.lock().expect("Internal error");
        *status = InstallEvent::InstallJava(progress.clone())
    }

    if instance.config.launch_config.java_path.is_none() && config.prefer_mojang_java {
        java::install_for_instance(&instance, &progress, config.download.clone()).await?;
    }

    if runtime.mod_loader_type.is_some() {
        info!("Install mod loader");
        {
            let mut status = status.lock().expect("Internal error");
            *status = InstallEvent::InstallModLoader(ModLoaderProgress::Prepare);
        }
        let reporter = ModLoaderReporter::new(&status);
        let installer_java = resolve_installer_java(&config, &instance).await?;
        install_mod_loader(runtime, installer_java.path.as_path(), &reporter).await?;
    };

    configure_first_launch_language(config, &instance).await;

    debug!("Saving lock file");
    tokio::fs::write(
        DATA_LOCATION
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
/// same selection logic as game launching (see `java_runtime::resolve_java_executable`).
async fn resolve_installer_java(
    config: &Config,
    instance: &Instance,
) -> Result<slint_java_runtime::ResolvedJava> {
    let minecraft_location = MinecraftLocation::new(&DATA_LOCATION.root);
    let version_json_path = minecraft_location.get_version_json(&instance.config.runtime.minecraft);
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await?;
    let unresolved_version = serde_json::from_str::<Version>(&raw_version_json)?;
    let resolved_version = resolve_version(&unresolved_version, &minecraft_location, &[]).await?;
    Ok(
        slint_java_runtime::resolve_java_executable(&slint_java_runtime::ResolveJavaOptions {
            instance_java_path: instance.config.launch_config.java_path.clone(),
            prefer_mojang_java: config.prefer_mojang_java,
            disabled_java_runtimes: config.disabled_java_runtime.clone(),
            required_major_version: resolved_version.java_version.major_version as u32,
            mojang_component: resolved_version.java_version.component.clone(),
        })
        .await?,
    )
}

/// Installs the specified mod loader for the provided runtime configuration.
///
/// # Arguments
/// * `runtime` - Instance runtime configuration containing loader type/version.
/// * `java_path` - The Java executable used to run Java-based installers
///   (Forge and NeoForge).
/// * `reporter` - Progress reporter forwarded to the loader installation.
///
/// # Errors
/// Returns an error if:
/// - The loader type/version is missing or malformed.
/// - The underlying installation function fails.
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
                MinecraftLocation::new(&DATA_LOCATION.root),
            )
            .await?
        }
        ModLoaderType::Quilt => {
            quilt::install(
                &runtime.minecraft,
                mod_loader_version,
                MinecraftLocation::new(&DATA_LOCATION.root),
            )
            .await?
        }
        ModLoaderType::Forge => {
            forge::install(
                &MinecraftLocation::new(&DATA_LOCATION.root),
                mod_loader_version,
                &runtime.minecraft,
                java_path,
                reporter,
            )
            .await?
        }
        ModLoaderType::Neoforge => {
            neoforge::install(&DATA_LOCATION.root, mod_loader_version, java_path, reporter).await?
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
    let options_txt_path = DATA_LOCATION
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
    let minecraft_location = MinecraftLocation::new(&DATA_LOCATION.root);
    let version_json_path = minecraft_location.get_version_json(&instance.config.runtime.minecraft);
    let raw_version_json = tokio::fs::read_to_string(version_json_path).await.ok()?;
    Version::from_str(&raw_version_json).ok()?.release_time
}
