use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::channel,
        Arc,
    },
    thread,
    time::Duration,
};

use aml_core::{
    core::{folder::MinecraftLocation, version::VersionManifest},
    install::{
        fabric::LoaderArtifactList,
        forge::version_list::ForgeVersionList,
        generate_download_list,
        quilt::{version_list::get_quilt_version_list_from_mcversion, QuiltVersion},
    },
    mod_parser::ResolvedMod,
    saves::level::LevelData,
    utils::download::Download,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use futures::StreamExt;
use notify::{watcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};

use crate::{Storage, DATA_LOCATION, HTTP_CLIENT, MAIN_WINDOW};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceConfig {
    pub name: String,
    pub runtime: InstanceRuntime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceRuntime {
    pub minecraft: String,
    pub fabric: String,
    pub forge: String,
    pub quilt: String,
    pub optifine: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Instance {
    pub config: InstanceConfig,
    pub installed: bool,
}

#[tauri::command(async)]
pub async fn get_minecraft_version_list() -> Option<VersionManifest> {
    VersionManifest::new().await.ok()
}

#[tauri::command(async)]
pub async fn get_fabric_version_list(mcversion: String) -> Option<LoaderArtifactList> {
    LoaderArtifactList::from_mcversion(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_forge_version_list(mcversion: String) -> Option<ForgeVersionList> {
    ForgeVersionList::from_mcversion(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_quilt_version_list(mcversion: String) -> Option<Vec<QuiltVersion>> {
    get_quilt_version_list_from_mcversion(None, &mcversion)
        .await
        .ok()
}

#[tauri::command(async)]
pub async fn create_instance(instance_name: String, config: InstanceConfig) -> Option<()> {
    async fn create_instance(instance_name: String, config: InstanceConfig) -> Result<()> {
        let data_location = DATA_LOCATION.get().unwrap();
        let instance_root = data_location.get_instance_root(&instance_name);
        let config_file = instance_root.join("instance.toml");
        fs::create_dir_all(config_file.parent().ok_or(anyhow::anyhow!("Path Error"))?).await?;
        fs::write(config_file, toml::to_string_pretty(&config)?).await?;
        Ok(())
    }
    create_instance(instance_name, config).await.ok()
}

#[tauri::command(async)]
pub async fn check_repeated_instance_name(instance_name: String) -> bool {
    let config_path = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(instance_name)
        .join("instance.toml");
    match config_path.metadata() {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[tauri::command(async)]
pub async fn scan_instances_folder() -> Option<Vec<Instance>> {
    println!("123");
    async fn scan() -> Result<Vec<Instance>> {
        let datafolder_path = DATA_LOCATION.get().unwrap();
        let instances_folder = &datafolder_path.instances;
        let mut folder_entries = tokio::fs::read_dir(instances_folder).await?;
        let mut results = Vec::new();
        while let Some(entry) = folder_entries.next_entry().await? {
            let file_type = match entry.file_type().await {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            let instance_config = path.join("instance.toml");
            if instance_config.metadata().is_err() || !instance_config.is_file() {
                continue;
            }
            let config_content = match fs::read_to_string(instance_config).await {
                Err(_) => continue,
                Ok(content) => content,
            };
            let config = match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => config,
                Err(_) => continue,
            };
            let fabric = config.runtime.fabric.as_str();
            let forge = config.runtime.forge.as_str();
            let quilt = config.runtime.quilt.as_str();
            if (!fabric.is_empty() && !forge.is_empty())
                || (!forge.is_empty() && !quilt.is_empty())
                || (!quilt.is_empty() && !fabric.is_empty())
            {
                continue;
            }
            results.push(Instance {
                config,
                installed: fs::File::open(path.join(".aml-ok")).await.is_ok(),
            })
        }
        Ok(results)
    }
    scan().await.ok()
}

#[tauri::command]
pub fn watch_instances_folder() {
    let main_window = MAIN_WINDOW.get().unwrap().clone();
    thread::spawn(move || {
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher
            .watch(
                &DATA_LOCATION.get().unwrap().instances,
                RecursiveMode::Recursive,
            )
            .unwrap();

        loop {
            match rx.recv() {
                Ok(event) => {
                    main_window
                        .emit("instances_changed", format!("{:#?}", event))
                        .unwrap();
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
}

#[tauri::command]
pub fn set_active_instance(instance_name: String, storage: tauri::State<Storage>) {
    let mut active_instance = storage.active_instance.lock().unwrap();
    *active_instance = instance_name;
}

#[tauri::command(async)]
pub async fn scan_mod_folder(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<Vec<ResolvedMod>, ()> {
    let instance_name = storage.active_instance.lock().unwrap().clone();

    async fn scan(
        instance_name: String,
        storage: tauri::State<'_, Storage>,
    ) -> Result<Vec<ResolvedMod>> {
        let data_location = DATA_LOCATION.get().unwrap();
        let mod_packs_root = data_location.get_modpacks_root(&instance_name);

        fs::create_dir_all(&mod_packs_root).await?;

        let mut folder_entries = tokio::fs::read_dir(mod_packs_root).await?;
        let mut results = Vec::new();
        while let Some(entry) = folder_entries.next_entry().await? {
            let file_type = match entry.file_type().await {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };
            let active_instance = storage.active_instance.lock().unwrap().clone();
            if &active_instance != &instance_name {
                return Err(anyhow!("stopped")); // if user change the active instance, stop scanning
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if path.metadata().is_err() {
                continue;
            }
            let parser_task = tokio::task::spawn_blocking(|| aml_core::mod_parser::parse_mod(path));

            results.push(match parser_task.await {
                Err(_) => continue,
                Ok(result) => match result {
                    Err(_) => continue,
                    Ok(result) => result,
                },
            });
        }
        Ok(results)
    }
    match scan(instance_name, storage).await {
        Ok(results) => Ok(results
            .into_iter()
            .filter(|v| v.version.is_some())
            .collect()),
        Err(_) => Err(()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saves {
    pub icon: String,
    pub level_data: LevelData,
    pub dir_name: String,
}

#[tauri::command(async)]
pub async fn scan_saves_folder(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<Vec<Saves>, ()> {
    let instance_name = storage.active_instance.lock().unwrap().clone();

    async fn scan(instance_name: String, storage: tauri::State<'_, Storage>) -> Result<Vec<Saves>> {
        let data_location = DATA_LOCATION.get().unwrap();
        let saves_root = data_location.get_saves_root(&instance_name);

        fs::create_dir_all(&saves_root).await?;

        let mut folder_entries = tokio::fs::read_dir(saves_root).await?;
        let mut results = Vec::new();
        while let Some(entry) = folder_entries.next_entry().await? {
            let file_type = match entry.file_type().await {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };
            let active_instance = storage.active_instance.lock().unwrap().clone();
            if &active_instance != &instance_name {
                return Err(anyhow!("stopped")); // if user change the active instance, stop scanning
            }
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            if path.metadata().is_err() {
                continue;
            }
            let level_path = path.join("level.dat");
            let parser_task =
                tokio::task::spawn_blocking(|| aml_core::saves::level::get_level_data(level_path));

            results.push(Saves {
                icon: {
                    let icon_path = path.join("icon.png");
                    match fs::read(icon_path).await {
                        Err(_) => "".to_string(),
                        Ok(content) => format!(
                            "data:image/png;base64,{}",
                            general_purpose::STANDARD_NO_PAD.encode(content)
                        ),
                    }
                },
                level_data: match parser_task.await {
                    Err(_) => continue,
                    Ok(result) => match result {
                        Err(_) => continue,
                        Ok(result) => result,
                    },
                },
                dir_name: path.file_name().unwrap().to_string_lossy().to_string(),
            });
        }
        Ok(results)
    }
    match scan(instance_name, storage).await {
        Ok(results) => Ok(results),
        Err(_) => Err(()),
    }
}

#[tauri::command(async)]
pub async fn get_instance_config(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<InstanceConfig, ()> {
    let instance_name = storage.active_instance.lock().unwrap().clone();
    let config_path = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(&instance_name)
        .join("instance.toml");
    match config_path.metadata() {
        Ok(_) => {
            let config_content = match fs::read_to_string(config_path).await {
                Err(_) => return Err(()),
                Ok(content) => content,
            };
            match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => Ok(config),
                Err(_) => Err(()),
            }
        }
        Err(_) => Err(()),
    }
}

#[tauri::command(async)]
pub async fn get_instance_config_from_string(instance_name: &str) -> Result<InstanceConfig, ()> {
    let config_path = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(&instance_name)
        .join("instance.toml");
    match config_path.metadata() {
        Ok(_) => {
            let config_content = match fs::read_to_string(config_path).await {
                Err(_) => return Err(()),
                Ok(content) => content,
            };
            match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => Ok(config),
                Err(_) => Err(()),
            }
        }
        Err(_) => Err(()),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadProgress {
    pub completed: usize,
    pub total: usize,
    pub step: usize,
}

async fn download_files(download_list: Vec<Download<String>>) {
    let main_window = MAIN_WINDOW.get().unwrap();
    main_window
        .emit(
            "install_progress",
            DownloadProgress {
                completed: 0,
                total: 0,
                step: 2,
            },
        )
        .unwrap();
    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let total = download_list.len();
    futures::stream::iter(download_list)
        .map(|task| {
            let counter = counter.clone();
            async move {
                let file_path = PathBuf::from(task.file);
                fs::create_dir_all(file_path.parent().unwrap())
                    .await
                    .unwrap();
                let mut response = HTTP_CLIENT
                    .get()
                    .unwrap()
                    .get(task.url)
                    .send()
                    .await
                    .unwrap();
                let mut file = fs::File::create(&file_path).await.unwrap();
                while let Some(chunk) = response.chunk().await.unwrap() {
                    file.write_all(&chunk).await.unwrap();
                }
                counter.fetch_add(1, Ordering::SeqCst);
            }
        })
        .buffer_unordered(16)
        .for_each_concurrent(None, |_| async {
            let counter = counter.clone().load(Ordering::SeqCst);
            println!("Progress: {counter} / {total}");
            main_window
                .emit(
                    "install_progress",
                    DownloadProgress {
                        completed: counter,
                        total,
                        step: 3,
                    },
                )
                .unwrap();
        })
        .await;
    if counter.load(Ordering::SeqCst) != total {
        main_window
            .emit(
                "install_progress",
                DownloadProgress {
                    completed: total,
                    total,
                    step: 4,
                },
            )
            .unwrap();
    }
}

#[tauri::command(async)]
pub async fn install_command(storage: tauri::State<'_, Storage>) -> std::result::Result<(), ()> {
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
    let active_instance = storage.active_instance.lock().unwrap().clone();
    let data_location = DATA_LOCATION.get().unwrap();
    let instance_config = get_instance_config_from_string(&active_instance)
        .await
        .unwrap();
    let runtime = instance_config.runtime;
    let download_list = generate_download_list(
        &runtime.minecraft,
        MinecraftLocation::new(&data_location.root),
    )
    .await
    .unwrap();
    download_files(download_list).await;
    let mut lock_file = fs::File::create(
        data_location
            .get_instance_root(active_instance)
            .join(".aml-ok"),
    )
    .await
    .unwrap();
    lock_file.write_all(b"ok").await.unwrap();
    Ok(())
}
