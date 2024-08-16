/*
 * Amethyst Launcher Core
 * Copyright (C) 2023 Broken-Deer <old_driver__@outlook.com> and contributors
 *
 * This program is free software, you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use forge::version_list::ForgeVersionList;
use quilt::version_list::QuiltVersionList;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use vanilla::generate_download_info;

use crate::{
    config::instance::{InstanceConfig, InstanceRuntime, ModLoaderType},
    download::{download_files, DownloadError, DownloadProgress},
    folder::{DataLocation, MinecraftLocation},
    version::VersionManifest,
    Storage, DATA_LOCATION, MAIN_WINDOW,
};

mod fabric;
mod forge;
mod quilt;
mod vanilla;

#[tauri::command(async)]
pub async fn get_minecraft_version_list() -> Option<VersionManifest> {
    VersionManifest::new().await.ok()
}

#[tauri::command(async)]
pub async fn get_fabric_version_list(mcversion: String) -> Option<fabric::LoaderArtifactList> {
    fabric::LoaderArtifactList::new(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_forge_version_list(mcversion: String) -> Option<ForgeVersionList> /* Option<ForgeVersionList> */
{
    ForgeVersionList::new(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_quilt_version_list(mcversion: String) -> Option<QuiltVersionList> {
    QuiltVersionList::from_mcversion(None, &mcversion)
        .await
        .ok()
}

#[tauri::command(async)]
pub async fn install(storage: tauri::State<'_, Storage>) -> std::result::Result<(), ()> {
    let main_window = MAIN_WINDOW.get().unwrap();
    main_window
        .emit(
            "install_progress",
            DownloadProgress {
                completed: 0,
                total: 0,
                step: 1,
            },
        )
        .unwrap();
    let active_instance = storage.current_instance.lock().unwrap().clone();
    let data_location = DATA_LOCATION.get().unwrap();
    let instance_config = match InstanceConfig::get(&active_instance).await {
        Ok(x) => x,
        Err(_) => {
            main_window
                .emit("install_error", DownloadError { step: 1 })
                .unwrap();
            return Err(());
        }
    };

    let runtime = instance_config.runtime;
    let download_list = match generate_download_info(
        &runtime.minecraft,
        MinecraftLocation::new(&data_location.root),
    )
    .await
    {
        Ok(x) => x,
        Err(_) => {
            main_window
                .emit("install_error", DownloadError { step: 1 })
                .unwrap();
            return Err(());
        }
    };

    download_files(download_list).await;
    if runtime.mod_loader_type.is_some() {
        main_window
            .emit(
                "install_progress",
                DownloadProgress {
                    completed: 0,
                    total: 0,
                    step: 4,
                },
            )
            .unwrap();
        match install_mod_loader(runtime, data_location).await {
            Ok(_) => (),
            Err(_) => return Err(()),
        };
    }
    let mut lock_file = tokio::fs::File::create(
        data_location
            .get_instance_root(active_instance)
            .join(".aml-ok"),
    )
    .await
    .unwrap();
    lock_file.write_all(b"ok").await.unwrap();
    main_window.emit("install_success", "").unwrap();
    Ok(())
}

async fn install_mod_loader(
    runtime: InstanceRuntime,
    data_location: &DataLocation,
) -> anyhow::Result<()> {
    let mod_loader_type = runtime.mod_loader_type.unwrap();
    let mod_loader_version = runtime
        .mod_loader_version
        .ok_or(anyhow::Error::msg("bad instance.toml file!"))?;
    match mod_loader_type {
        ModLoaderType::Fabric => {
            fabric::install(fabric::install::FabricInstallOptions {
                mcversion: runtime.minecraft,
                install_dir: data_location.root.clone(),
                loader: mod_loader_version,
            })
            .await?;
        }
        ModLoaderType::Quilt => {}
        ModLoaderType::Forge => {}
        ModLoaderType::Neoforge => {}
    }

    anyhow::Ok(())
}
