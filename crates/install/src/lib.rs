// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
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

use forge::ForgeVersionList;
use futures::future::{AbortHandle, Abortable};
use log::{debug, info};
use neoforged::NeoforgedVersionList;
use quilt::QuiltVersionList;
use serde::Serialize;
use tauri::{
    Manager, Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};
use vanilla::generate_download_info;

use config::{
    Config,
    instance::{InstanceRuntime, ModLoaderType},
};
use download::download_concurrent;
use download::task::Progress;
use folder::{DATA_LOCATION, MinecraftLocation};
use instance::Instance;

use crate::vanilla::VersionManifest;

mod error;
pub mod fabric;
pub mod forge;
pub mod java;
pub mod neoforged;
pub mod quilt;
pub mod vanilla;

pub use error::*;

#[derive(Clone, Default)]
struct PluginState {
    current_task: Arc<Mutex<Option<AbortHandle>>>,
    version_manifest_cache: Arc<Mutex<Option<(u64, VersionManifest)>>>,
    fabric_version_list_cache: Arc<Mutex<Option<(u64, fabric::LoaderArtifactList)>>>,
    quilt_version_list_cache: Arc<Mutex<Option<(u64, QuiltVersionList)>>>,
    forge_version_list_cache: Arc<Mutex<Option<(u64, ForgeVersionList)>>>,
    #[allow(clippy::type_complexity)]
    neoforged_version_list_cache: Arc<Mutex<Option<(u64, Vec<String>)>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("install")
        .invoke_handler(tauri::generate_handler![
            cmd_get_minecraft_version_list,
            cmd_get_fabric_version_list,
            cmd_get_quilt_version_list,
            cmd_get_forge_version_list,
            cmd_get_neoforged_version_list,
            cmd_create_install_task,
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
        && now - cache.0 > 2 * 60 * 60
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
async fn cmd_get_fabric_version_list(
    state: State<'_, PluginState>,
    mcversion: String,
) -> Result<fabric::LoaderArtifactList> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .fabric_version_list_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > 2 * 60 * 60
    {
        return Ok(cache.1);
    }
    let result = fabric::LoaderArtifactList::new(&mcversion).await?;
    {
        let mut cache = state
            .fabric_version_list_cache
            .lock()
            .expect("Internal error");
        *cache = Some((now, result.clone()))
    }
    Ok(result)
}

#[command]
async fn cmd_get_forge_version_list(
    state: State<'_, PluginState>,
    mcversion: String,
) -> Result<ForgeVersionList> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .forge_version_list_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > 2 * 60 * 60
    {
        return Ok(cache.1);
    }
    let result = ForgeVersionList::new(&mcversion).await?;
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
async fn cmd_get_quilt_version_list(
    state: State<'_, PluginState>,
    mcversion: String,
) -> Result<QuiltVersionList> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .quilt_version_list_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > 2 * 60 * 60
    {
        return Ok(cache.1);
    }
    let result = QuiltVersionList::new(&mcversion).await?;
    {
        let mut cache = state
            .quilt_version_list_cache
            .lock()
            .expect("Internal error");
        *cache = Some((now, result.clone()))
    }
    Ok(result)
}

#[command]
async fn cmd_get_neoforged_version_list(
    state: State<'_, PluginState>,
    mcversion: String,
) -> Result<Vec<String>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect System Time")
        .as_secs();
    if let Some(cache) = state
        .neoforged_version_list_cache
        .lock()
        .expect("Internal error")
        .clone()
        && now - cache.0 > 2 * 60 * 60
    {
        return Ok(cache.1);
    }
    let result = NeoforgedVersionList::from_mcversion(&mcversion).await?;
    {
        let mut cache = state
            .neoforged_version_list_cache
            .lock()
            .expect("Internal error");
        *cache = Some((now, result.clone()))
    }
    Ok(result)
}

#[command]
async fn cmd_create_install_task(
    state: State<'_, PluginState>,
    config: Config,
    instance: Instance,
    channel: Channel<InstallEvent>,
) -> Result<()> {
    if state.current_task.lock().expect("Internal error").is_some() {
        return Ok(());
    }
    let (handle, reg) = AbortHandle::new_pair();
    let future = Abortable::new(install(config, instance, channel), reg);
    {
        let mut current_task = state.current_task.lock().expect("Internal error");
        *current_task = Some(handle);
    }
    future.await?
}

#[command]
async fn cmd_cancel_install_task(state: State<'_, PluginState>) -> Result<()> {
    let mut current_task = state.current_task.lock().expect("Internal error");
    if let Some(handle) = current_task.clone() {
        handle.abort();
    }
    *current_task = None;
    Ok(())
}

#[derive(Clone, Serialize)]
pub enum InstallJob {
    Prepare,
    InstallGame,
    InstallJava,
    InstallModLoader,
}

// TODO: Use enum, like launch crate
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallEvent {
    completed: u64,
    total: u64,
    downloader_step: download::task::Step,
    speed: u64,
    job: InstallJob,
}

/// Installs Minecraft, Java, and optionally a mod loader for the given instance.
///
/// This function runs a full installation pipeline including:
/// - Downloading Minecraft game files
/// - Installing Java
/// - Installing a mod loader (Fabric, Forge, Quilt, NeoForged)
///
/// # Arguments
/// * `storage` - Shared application storage (configuration).
/// * `instance` - The instance configuration.
///
/// Emits `"install_success"` on completion.
pub async fn install(
    config: Config,
    instance: Instance,
    event_channel: Channel<InstallEvent>,
) -> Result<()> {
    let progress = Progress::default();
    let finished = Arc::new(AtomicBool::new(false));
    let install_job = Arc::new(Mutex::new(InstallJob::Prepare));
    // FIXME: If download failed, this will never stop, it will be a zombie thread. Move this
    // sender to cmd_ function to fix it.
    let progress_sender_thread = {
        let progress_cloned = progress.clone();
        let finished = finished.clone();
        let install_job_cloned = install_job.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                let downloader_step =
                    { progress_cloned.step.lock().expect("Internal Error").clone() };
                let job = { install_job_cloned.lock().expect("Internal Error").clone() };
                let _ = event_channel.send(InstallEvent {
                    completed: progress_cloned.completed.load(Ordering::SeqCst),
                    total: progress_cloned.total.load(Ordering::SeqCst),
                    speed: progress_cloned.speed.load(Ordering::SeqCst),
                    downloader_step,
                    job,
                });
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    // TODO: send state preparing installation
    info!(
        "Start installing the game for instance {}",
        instance.config.name
    );
    let runtime = instance.config.runtime;
    info!("------------- Instance runtime config -------------");
    info!("-> Minecraft: {}", runtime.minecraft);
    match &runtime.mod_loader_type {
        Some(x) => info!("-> Mod loader: {x}"),
        None => info!("-> Mod loader: none"),
    };
    match &runtime.mod_loader_version {
        Some(x) => info!("-> Mod loader version: {x}"),
        None => info!("-> Mod loader version: none"),
    };
    info!("Generating download task...");
    let download_list = generate_download_info(
        &runtime.minecraft,
        MinecraftLocation::new(&DATA_LOCATION.root),
    )
    .await?;
    info!("Start downloading file");
    download_concurrent(download_list, &progress, config.download).await?;
    info!("Installing Java");
    // TODO: Send state install java
    let java_version_list = java::MojangJavaVersionList::new().await?;
    let java_for_current_platform = java_version_list
        .get_current_platform()
        .ok_or(Error::NoSupportedJavaRuntime)?;
    java::group_install(&DATA_LOCATION.root.join("java"), java_for_current_platform).await?;
    if runtime.mod_loader_type.is_some() {
        info!("Install mod loader");
        //TODO: Send state install mod loader
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
    let _ = progress_sender_thread.join();
    Ok(())
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
pub async fn install_mod_loader(runtime: InstanceRuntime) -> Result<()> {
    let mod_loader_type = runtime.mod_loader_type.ok_or(Error::InstanceBroken)?;
    let mod_loader_version = runtime.mod_loader_version.ok_or(Error::InstanceBroken)?;
    match mod_loader_type {
        ModLoaderType::Fabric => {
            fabric::install(
                &runtime.minecraft,
                &mod_loader_version,
                MinecraftLocation::new(&DATA_LOCATION.root),
            )
            .await?
        }
        ModLoaderType::Quilt => {
            quilt::install(
                &runtime.minecraft,
                &mod_loader_version,
                MinecraftLocation::new(&DATA_LOCATION.root),
            )
            .await?
        }
        ModLoaderType::Forge => {
            forge::install(&DATA_LOCATION.root, &mod_loader_version, &runtime.minecraft).await?
        }
        ModLoaderType::Neoforged => {
            neoforged::install(&DATA_LOCATION.root, &mod_loader_version).await?
        }
    }

    Ok(())
}
