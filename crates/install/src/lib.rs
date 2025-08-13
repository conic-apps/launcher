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
use shared::MAIN_WINDOW;
use tauri::{
    Emitter, Runtime, command,
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

pub mod fabric;
pub mod forge;
pub mod java;
pub mod neoforged;
pub mod quilt;
pub mod vanilla;

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
async fn cmd_get_minecraft_version_list() -> Option<VersionManifest> {
    // TODO: Use cache, 2 hours
    VersionManifest::new().await.ok()
}

#[command]
async fn cmd_get_fabric_version_list(mcversion: String) -> Option<fabric::LoaderArtifactList> {
    //TODO: all error handle, avoid use anyhow
    fabric::LoaderArtifactList::new(&mcversion).await.ok()
}

#[command]
async fn cmd_get_forge_version_list(mcversion: String) -> Option<ForgeVersionList> {
    ForgeVersionList::new(&mcversion).await.ok()
}

#[command]
async fn cmd_get_quilt_version_list(mcversion: String) -> Option<QuiltVersionList> {
    QuiltVersionList::new(&mcversion).await.ok()
}

#[command]
async fn cmd_get_neoforged_version_list(mcversion: String) -> Option<Vec<String>> {
    NeoforgedVersionList::from_mcversion(&mcversion).await.ok()
}

#[command]
async fn cmd_install(config: Config, instance: Instance) -> Option<()> {
    install(config, instance).await.ok()
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
pub async fn install(config: Config, instance: Instance) -> std::result::Result<(), ()> {
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
        #[allow(clippy::unwrap_used)]
        let mut task = progress.task.lock().unwrap();
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
    .await
    .unwrap();
    info!("Start downloading file");
    download_concurrent(download_list, &progress, config.download)
        .await
        .unwrap();
    info!("Installing Java");
    {
        #[allow(clippy::unwrap_used)]
        let mut task = progress.task.lock().unwrap();
        *task = Task::InstallJava;
    }
    let java_version_list = java::MojangJavaVersionList::new().await.unwrap();
    let java_for_current_platform = java_version_list.get_current_platform().unwrap();
    java::group_install(&DATA_LOCATION.root.join("java"), java_for_current_platform).await;
    if runtime.mod_loader_type.is_some() {
        info!("Install mod loader");
        {
            #[allow(clippy::unwrap_used)]
            let mut task = progress.task.lock().unwrap();
            *task = Task::InstallModLoader;
        };
        install_mod_loader(runtime).await.unwrap();
    };
    debug!("Saving lock file");
    async_fs::write(
        DATA_LOCATION
            .get_instance_root(&instance.id)
            .join(".install.lock"),
        b"ok",
    )
    .await
    .unwrap();
    MAIN_WINDOW.emit("install_success", "").unwrap();
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
pub async fn install_mod_loader(runtime: InstanceRuntime) -> anyhow::Result<()> {
    let mod_loader_type = runtime.mod_loader_type.unwrap();
    let mod_loader_version = runtime
        .mod_loader_version
        .ok_or(anyhow::Error::msg("bad instance.toml file!"))?;
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

    anyhow::Ok(())
}
