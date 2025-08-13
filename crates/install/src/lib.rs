// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// TODO: Support Optifine auto install

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use download::download_concurrent;
use forge::ForgeVersionList;
use log::{debug, info};
use neoforged::NeoforgedVersionList;
use quilt::QuiltVersionList;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use vanilla::generate_download_info;

use config::{
    Config,
    instance::{InstanceRuntime, ModLoaderType},
};
use folder::{DATA_LOCATION, MinecraftLocation};
use instance::Instance;
use task::{Progress, Task};

use crate::vanilla::VersionManifest;

pub mod error;
pub mod fabric;
pub mod forge;
pub mod java;
pub mod neoforged;
pub mod quilt;
pub mod vanilla;

use error::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("install")
        .invoke_handler(tauri::generate_handler![
            cmd_get_minecraft_version_list,
            cmd_get_fabric_version_list,
            cmd_get_quilt_version_list,
            cmd_get_forge_version_list,
            cmd_get_neoforged_version_list,
            cmd_install
        ])
        .build()
}

#[command]
async fn cmd_get_minecraft_version_list() -> Result<VersionManifest> {
    // TODO: Use cache, 2 hours
    VersionManifest::new().await
}

#[command]
async fn cmd_get_fabric_version_list(mcversion: String) -> Result<fabric::LoaderArtifactList> {
    //TODO: all error handle, avoid use anyhow
    fabric::LoaderArtifactList::new(&mcversion).await
}

#[command]
async fn cmd_get_forge_version_list(mcversion: String) -> Result<ForgeVersionList> {
    ForgeVersionList::new(&mcversion).await
}

#[command]
async fn cmd_get_quilt_version_list(mcversion: String) -> Result<QuiltVersionList> {
    QuiltVersionList::new(&mcversion).await
}

#[command]
async fn cmd_get_neoforged_version_list(mcversion: String) -> Result<Vec<String>> {
    NeoforgedVersionList::from_mcversion(&mcversion).await
}

#[command]
async fn cmd_install(config: Config, instance: Instance) -> Result<()> {
    install(config, instance).await
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
pub async fn install(config: Config, instance: Instance) -> Result<()> {
    let progress = Progress::default();
    let finished = Arc::new(AtomicBool::new(false));
    let progress_sender_thread = {
        let progress = progress.clone();
        let finished = finished.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                progress.send();
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    {
        let mut task = progress
            .task
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = Task::PrepareInstallGame;
    }
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
    {
        let mut task = progress
            .task
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = Task::InstallJava;
    }
    let java_version_list = java::MojangJavaVersionList::new().await?;
    let java_for_current_platform = java_version_list
        .get_current_platform()
        .ok_or(Error::NoSupportedJavaRuntime)?;
    java::group_install(&DATA_LOCATION.root.join("java"), java_for_current_platform).await?;
    if runtime.mod_loader_type.is_some() {
        info!("Install mod loader");
        {
            let mut task = progress
                .task
                .lock()
                .expect("Internal error: another thread hold lock and panic");
            *task = Task::InstallModLoader;
        };
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
