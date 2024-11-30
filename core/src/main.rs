// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod account;
mod config;
mod download;
mod folder;
pub mod game_data;
mod install;
mod instance;
mod launch;
mod platform;
pub mod utils;
mod version;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::account::{
    add_microsoft_account, delete_account, get_account_by_uuid, get_accounts,
    refresh_all_microsoft_account, refresh_microsoft_account_by_uuid,
};
use crate::config::instance::{get_instance_config, get_instance_config_by_name};
use crate::game_data::{scan_mod_folder, scan_resourcepack_folder, scan_saves_folder};
use crate::install::install;
use crate::install::{
    get_fabric_version_list, get_forge_version_list, get_minecraft_version_list,
    get_quilt_version_list,
};
use crate::instance::{
    check_instance_existance, create_instance, delete_instance, scan_instances_folder,
    set_current_instance,
};
use crate::launch::launch;
use config::{read_config_file, save_config, update_config, Config};
use folder::DataLocation;
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use platform::{OsType, PlatformInfo};
use tauri::{Emitter, Manager, Window};
use tauri_plugin_http::reqwest;
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tauri_plugin_log::{Target, TargetKind};
use version::VersionManifest;

/// use MAIN_WINDOW.emit() to send message to main window
static MAIN_WINDOW: OnceCell<Window> = OnceCell::new();
static APP_VERSION: OnceCell<String> = OnceCell::new();
static DATA_LOCATION: OnceCell<DataLocation> = OnceCell::new();
static PLATFORM_INFO: OnceCell<PlatformInfo> = OnceCell::new();
static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static APPLICATION_DATA: OnceCell<PathBuf> = OnceCell::new();
const DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("../assets/launcher_profiles.json");

pub struct Storage {
    pub current_instance: Arc<Mutex<String>>,
    pub config: Arc<Mutex<Config>>,
}

#[tokio::main]
async fn main() {
    PLATFORM_INFO.set(PlatformInfo::new().await).unwrap();
    initialize_application_data().await;
    let data_location = DataLocation::new(APPLICATION_DATA.get().unwrap());
    tokio::fs::create_dir_all(&data_location.root)
        .await
        .expect("Could not create appliaction data folder");
    DATA_LOCATION.set(data_location).unwrap();
    HTTP_CLIENT
        .set(
            reqwest::ClientBuilder::new()
                .pool_idle_timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(100)
                .build()
                .unwrap(),
        )
        .unwrap();
    let launcher_profiles_path = DATA_LOCATION
        .get()
        .unwrap()
        .root
        .join("launcher_profiles.json");
    let _ = tokio::fs::remove_file(&launcher_profiles_path).await;
    tokio::fs::write(&launcher_profiles_path, DEFAULT_LAUNCHER_PROFILE)
        .await
        .expect("Could not create launcher profile");
    tokio::spawn(instance::update_latest_instance());
    #[cfg(target_os = "linux")]
    {
        // if std::path::Path::new("/dev/dri").exists()
        //     && std::env::var("WAYLAND_DISPLAY").is_err()
        //     && std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "x11"
        // {
        // SAFETY: There's potential for race conditions in a multi-threaded context.
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        // }
    }
    let config = read_config_file();
    let init_config_js_script = "
        Object.defineProperty(window, '__APPLICATION_CONFIG__', {
            value: JSON.parse(`"
        .to_string()
        + serde_json::to_string_pretty(&config).unwrap().as_ref()
        + "`)
        })
    "
        .to_string()
        .as_ref();
    tauri::Builder::default()
        .plugin(init_log_builder().build())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let windows = app.webview_windows();
            windows
                .values()
                .next()
                .expect("Sorry, no window found")
                .set_focus()
                .expect("Can't Bring Window to Focus");
        }))
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            create_instance,
            delete_instance,
            get_minecraft_version_list,
            get_fabric_version_list,
            get_forge_version_list,
            get_quilt_version_list,
            check_instance_existance,
            scan_instances_folder,
            set_current_instance,
            scan_mod_folder,
            scan_saves_folder,
            scan_resourcepack_folder,
            get_instance_config,
            get_instance_config_by_name,
            install,
            launch,
            read_config_file,
            update_config,
            save_config,
            on_frontend_loaded,
            add_microsoft_account,
            get_accounts,
            refresh_microsoft_account_by_uuid,
            refresh_all_microsoft_account,
            delete_account,
            get_account_by_uuid
        ])
        .manage(Storage {
            current_instance: Arc::new(Mutex::new("".to_string())),
            config: Arc::new(Mutex::new(config.clone())),
        })
        .append_invoke_initialization_script(init_config_js_script)
        .setup(move |app| {
            print_title();
            MAIN_WINDOW.set(app.get_window("main").unwrap()).unwrap();
            APP_VERSION
                .set(app.package_info().version.to_string())
                .unwrap();
            info!("Main window loaded");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Do something after app closed
            if window.label() != "main" {
                return;
            };
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.close().unwrap();
                match std::fs::remove_dir_all(&DATA_LOCATION.get().unwrap().temp) {
                    Ok(_) => info!("Temporary files cleared"),
                    Err(x) => {
                        if x.kind() != std::io::ErrorKind::NotFound {
                            error!("Could not clear temp foler")
                        }
                    }
                };
            }
        })
        .run(tauri::generate_context!())
        .expect("Failed to run app");
}

async fn initialize_application_data() {
    let platform_info = PLATFORM_INFO.get().unwrap();
    match platform_info.os_type {
        OsType::Linux => info!("The program is running on Linux {}", platform_info.version),
        OsType::Osx => info!("The program is running on macOS {}", platform_info.version),
        OsType::Windows => info!(
            "The program is running on fucking Windows {}",
            platform_info.version
        ),
    }
    #[cfg(not(debug_assertions))]
    let application_folder_name = "aml";
    #[cfg(debug_assertions)]
    let application_folder_name = "aml-debug";
    match platform_info.os_type {
        OsType::Windows => {
            APPLICATION_DATA
                .set(
                    PathBuf::from(
                        std::env::var("APPDATA").expect("Could not found APP_DATA directory"),
                    )
                    .join(application_folder_name),
                )
                .unwrap();
        }
        OsType::Linux => {
            APPLICATION_DATA
                .set(
                    PathBuf::from(std::env::var("HOME").expect("Could not found home"))
                        .join(format!(".{}", application_folder_name)),
                )
                .unwrap();
        }
        OsType::Osx => {
            APPLICATION_DATA
                .set(PathBuf::from("/Users/").join(application_folder_name))
                .unwrap();
        }
    }
    info!(
        "Application data path: {}",
        APPLICATION_DATA.get().unwrap().to_string_lossy()
    );
}

#[tauri::command(async)]
async fn on_frontend_loaded(storage: tauri::State<'_, Storage>) -> std::result::Result<(), ()> {
    info!("Frontend loaded");
    let config = &storage.config.lock().unwrap().clone();
    let _ = remind_minecraft_latest(config).await;
    Ok(())
}

async fn remind_minecraft_latest(config: &Config) -> anyhow::Result<()> {
    let data_location = DATA_LOCATION.get().unwrap();
    let (latest, cache_file) = if config.accessibility.snapshot_reminder {
        let latest = VersionManifest::new().await?.latest.snapshot;
        let cache_file = data_location.cache.join("latest_release");
        (latest, cache_file)
    } else if config.accessibility.release_reminder {
        let latest = VersionManifest::new().await?.latest.release;
        let cache_file = data_location.cache.join("latest_snapshot");
        (latest, cache_file)
    } else {
        return Ok(());
    };
    let cache = tokio::fs::read_to_string(&cache_file).await?;
    tokio::fs::write(&cache_file, &latest).await?;
    if latest != cache {
        let _ = MAIN_WINDOW.get().unwrap().emit("remind_update", latest);
    }
    Ok(())
}

fn init_log_builder() -> tauri_plugin_log::Builder {
    let log_builder = tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
            Target::new(TargetKind::Folder {
                path: DATA_LOCATION.get().unwrap().logs.clone(),
                file_name: None,
            }),
        ])
        .max_file_size(50_000)
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll);
    #[cfg(debug_assertions)]
    let log_builder = log_builder.with_colors(ColoredLevelConfig {
        error: Color::Red,
        warn: Color::Yellow,
        info: Color::Green,
        debug: Color::Blue,
        trace: Color::Cyan,
    });
    log_builder
}

fn print_title() {
    debug!("  █████╗ ███╗   ███╗███████╗████████╗██╗  ██╗██╗   ██╗███████╗████████╗ ");
    debug!(" ██╔══██╗████╗ ████║██╔════╝╚══██╔══╝██║  ██║╚██╗ ██╔╝██╔════╝╚══██╔══╝ ");
    debug!(" ███████║██╔████╔██║█████╗     ██║   ███████║ ╚████╔╝ ███████╗   ██║    ");
    debug!(" ██╔══██║██║╚██╔╝██║██╔══╝     ██║   ██╔══██║  ╚██╔╝  ╚════██║   ██║    ");
    debug!(" ██║  ██║██║ ╚═╝ ██║███████╗   ██║   ██║  ██║   ██║   ███████║   ██║    ");
    debug!(" ╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝   ╚══════╝   ╚═╝    ");
    debug!("");
    debug!(" ██╗      █████╗ ██╗   ██╗███╗   ██╗ ██████╗██╗  ██╗███████╗██████╗     ");
    debug!(" ██║     ██╔══██╗██║   ██║████╗  ██║██╔════╝██║  ██║██╔════╝██╔══██╗    ");
    debug!(" ██║     ███████║██║   ██║██╔██╗ ██║██║     ███████║█████╗  ██████╔╝    ");
    debug!(" ██║     ██╔══██║██║   ██║██║╚██╗██║██║     ██╔══██║██╔══╝  ██╔══██╗    ");
    debug!(" ███████╗██║  ██║╚██████╔╝██║ ╚████║╚██████╗██║  ██║███████╗██║  ██║    ");
    debug!(" ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝    ");
    info!("Amethyst Launcher is starting up");
    info!("Amethyst Launcher is open source, You can view the source code on Github: https://github.com/Conic-Sections/Amethyst-Launcher");
}
