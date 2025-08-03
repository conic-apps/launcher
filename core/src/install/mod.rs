// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use forge::version_list::ForgeVersionList;
use log::{debug, info};
use neoforged::NeoforgedVersionList;
use quilt::QuiltVersionList;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use vanilla::generate_download_info;

use crate::{
    config::instance::{InstanceRuntime, ModLoaderType},
    download::download_files,
    folder::MinecraftLocation,
    instance::Instance,
    task::{Progress, Task},
    version::VersionManifest,
    Storage, DATA_LOCATION, MAIN_WINDOW,
};

mod fabric;
mod forge;
mod java;
mod neoforged;
mod quilt;
pub mod vanilla;

#[tauri::command(async)]
pub async fn get_minecraft_version_list() -> Option<VersionManifest> {
    VersionManifest::new().await.ok()
}

#[tauri::command(async)]
pub async fn get_fabric_version_list(mcversion: String) -> Option<fabric::LoaderArtifactList> {
    fabric::LoaderArtifactList::new(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_forge_version_list(mcversion: String) -> Option<ForgeVersionList> {
    ForgeVersionList::new(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_quilt_version_list(mcversion: String) -> Option<QuiltVersionList> {
    QuiltVersionList::new(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_neoforged_version_list(mcversion: String) -> Option<Vec<String>> {
    let version_list = NeoforgedVersionList::new().await.ok()?;
    let splited_mcversion: Vec<&str> = mcversion.split('.').collect();
    Some(
        version_list
            .versions
            .into_iter()
            .rev()
            .filter(|x| {
                let splited_version: Vec<&str> = x.split('.').collect();
                #[allow(clippy::get_first)]
                return splited_version.get(0) == splited_mcversion.get(1)
                    && (splited_version.get(1) == splited_mcversion.get(2)
                        || (splited_version.get(1) == Some(&"0")
                            && splited_mcversion.get(2).is_none()));
            })
            .collect(),
    )
}

#[tauri::command(async)]
pub async fn install(
    storage: tauri::State<'_, Storage>,
    instance: Instance,
) -> std::result::Result<(), ()> {
    let progress = Progress::new();
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
    let config = storage.config.lock().unwrap().clone();
    download_files(download_list, &progress, config.download, false)
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
    let mut lock_file = tokio::fs::File::create(
        DATA_LOCATION
            .get_instance_root(&instance.id)
            .join(".install.lock"),
    )
    .await
    .unwrap();
    lock_file.write_all(b"ok").await.unwrap();
    MAIN_WINDOW.emit("install_success", "").unwrap();
    let _ = progress_sender_thread.join();
    Ok(())
}

async fn install_mod_loader(runtime: InstanceRuntime) -> anyhow::Result<()> {
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
