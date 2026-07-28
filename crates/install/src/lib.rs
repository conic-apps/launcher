// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// TODO: Support Optifine auto install

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{debug, info, warn};
use neoforge::NeoforgeVersionList;
use quilt::QuiltVersionList;
use serde::Serialize;
use tauri::{
    Manager, Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};
use vanilla::generate_download_info;

use config::Config;
use download::download_concurrent;
use download::progress::DownloadState;
use folder::{DATA_LOCATION, MinecraftLocation};
use instance::{Instance, InstanceRuntime, ModLoaderType};

use crate::{forge::ForgeVersionList, vanilla::VersionManifest};

pub mod authlib_injector;
mod error;
pub mod fabric;
pub mod forge;
pub mod java;
pub mod neoforge;
pub mod quilt;
pub mod vanilla;

pub use error::*;

static CACHE_EXPIRATION_SECONDS: u64 = 1800;

#[derive(Clone, Default)]
struct PluginState {
    task: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    version_manifest_cache: Arc<Mutex<Option<(u64, VersionManifest)>>>,
    forge_version_list_cache: Arc<Mutex<Option<(u64, ForgeVersionList)>>>,
    #[allow(clippy::type_complexity)]
    neoforge_version_list_cache: Arc<Mutex<Option<(u64, Vec<String>)>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("install")
        .invoke_handler(tauri::generate_handler![
            cmd_get_minecraft_version_list,
            cmd_get_fabric_version_list,
            cmd_get_quilt_version_list,
            cmd_get_forge_version_list,
            cmd_get_neoforge_version_list,
            cmd_spawn_install_task,
            cmd_cancel_install_task,
        ])
        .setup(|app, _| {
            app.manage(PluginState::default());
            Ok(())
        })
        .build()
}

#[command]
async fn cmd_get_minecraft_version_list(state: State<'_, PluginState>) -> Result<VersionManifest> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .version_manifest_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > CACHE_EXPIRATION_SECONDS
    {
        return Ok(cache.1);
    }
    let result = VersionManifest::new().await?;
    {
        let mut cache = state.version_manifest_cache.lock().expect("Internal error");
        *cache = Some((now, result.clone()))
    }
    Ok(result)
}

#[command]
async fn cmd_get_fabric_version_list(mcversion: String) -> Result<fabric::LoaderArtifactList> {
    fabric::LoaderArtifactList::new(&mcversion).await
}

#[command]
async fn cmd_get_forge_version_list(state: State<'_, PluginState>) -> Result<ForgeVersionList> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .forge_version_list_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > CACHE_EXPIRATION_SECONDS
    {
        return Ok(cache.1);
    }
    let result = ForgeVersionList::new().await?;
    {
        let mut cache = state
            .forge_version_list_cache
            .lock()
            .expect("Internal error");
        *cache = Some((now, result.clone()))
    }
    Ok(result)
}

#[command]
async fn cmd_get_quilt_version_list(mcversion: String) -> Result<QuiltVersionList> {
    QuiltVersionList::new(&mcversion).await
}

#[command]
async fn cmd_get_neoforge_version_list(
    state: State<'_, PluginState>,
    mcversion: String,
) -> Result<Vec<String>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .neoforge_version_list_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > CACHE_EXPIRATION_SECONDS
    {
        return Ok(cache.1);
    }
    let result = NeoforgeVersionList::from_mcversion(&mcversion).await?;
    {
        let mut cache = state
            .neoforge_version_list_cache
            .lock()
            .expect("Internal error");
        *cache = Some((now, result.clone()))
    }
    Ok(result)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "job", content = "downloadState")]
pub enum InstallEvent {
    Prepare,
    InstallGame(DownloadState),
    InstallJava(DownloadState),
    InstallModLoader,
}

#[command]
async fn cmd_spawn_install_task(
    state: State<'_, PluginState>,
    config: Config,
    instance: Instance,
    channel: Channel<InstallEvent>,
) -> Result<()> {
    if state.task.lock().expect("Internal error").is_some() {
        return Err(Error::AlreadyInstalling);
    }
    let task_status = Arc::new(Mutex::new(InstallEvent::Prepare));
    let finished = Arc::new(AtomicBool::new(false));
    let handle = tokio::spawn({
        let task_status_cloned = task_status.clone();
        let finished = finished.clone();
        async move {
            let result = install(config, instance, task_status_cloned).await;
            finished.store(true, Ordering::SeqCst);
            result
        }
    });
    {
        let mut current_task = state.task.lock().expect("Internal error");
        *current_task = Some(handle.abort_handle());
    }
    let event_sender_thread = {
        let status_cloned = task_status.clone();
        let finished = finished.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                let _ = channel.send(status_cloned.lock().expect("Internal error").clone());
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    let result = match handle.await {
        Ok(result) => result,
        Err(error) => {
            warn!("Installation cancelled");
            Err(Error::Aborted(error))
        }
    };
    let _ = event_sender_thread.join();
    {
        let mut current_task = state.task.lock().expect("Internal error");
        *current_task = None
    }
    result
}

#[command]
fn cmd_cancel_install_task(state: State<'_, PluginState>) {
    let mut current_task = state.task.lock().expect("Internal error");
    if let Some(handle) = current_task.clone() {
        handle.abort();
        warn!("Cancelling installation!");
    }
    *current_task = None;
}

/// Installs Minecraft, Java, and optionally a mod loader for the given instance.
///
/// This function runs a full installation pipeline including:
/// - Downloading Minecraft game files
/// - Installing Java
/// - Installing a mod loader (Fabric, Forge, Quilt, NeoForge)
///
/// # Arguments
/// * `storage` - Shared application storage (configuration).
/// * `instance` - The instance configuration.
///
/// Emits `"install_success"` on completion.
pub async fn install(
    config: Config,
    instance: Instance,
    status: Arc<Mutex<InstallEvent>>,
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

    if instance.config.launch_config.java_path.is_none() {
        java::install_for_instance(&instance, &progress, config.download.clone()).await?;
    }

    if runtime.mod_loader_type.is_some() {
        info!("Install mod loader");
        {
            let mut status = status.lock().expect("Internal error");
            *status = InstallEvent::InstallModLoader;
        }
        // TODO: mod loader installation progress
        install_mod_loader(runtime).await?;
    };

    debug!("Saving lock file");
    async_fs::write(
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

/// Installs the specified mod loader for the provided runtime configuration.
///
/// # Arguments
/// * `runtime` - Instance runtime configuration containing loader type/version.
///
/// # Errors
/// Returns an error if:
/// - The loader type/version is missing or malformed.
/// - The underlying installation function fails.
pub async fn install_mod_loader(runtime: &InstanceRuntime) -> Result<()> {
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
            )
            .await?
        }
        ModLoaderType::Neoforge => {
            neoforge::install(&DATA_LOCATION.root, mod_loader_version).await?
        }
    }

    Ok(())
}
