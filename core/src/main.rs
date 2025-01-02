// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// #![deny(clippy::unwrap_used)]

mod account;
mod config;
mod download;
mod folder;
// mod game_data;
mod install;
mod instance;
mod launch;
mod platform;
pub mod utils;
mod version;

use std::panic::{set_hook, PanicHookInfo};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use config::{read_config_file, Config};
use folder::DataLocation;
use log::{debug, error, info};
use once_cell::sync::{Lazy, OnceCell};
use platform::{OsType, PlatformInfo};
use tauri::{AppHandle, Emitter, Manager, Window};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_http::reqwest;
#[cfg(debug_assertions)]
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tauri_plugin_log::{Target, TargetKind};
use version::VersionManifest;

/// use MAIN_WINDOW.emit() to send message to main window
static APP_VERSION: OnceCell<String> = OnceCell::new();
static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
static MAIN_WINDOW: Lazy<Window> =
    Lazy::new(|| APP_HANDLE.get().unwrap().get_window("main").unwrap());
static DATA_LOCATION: Lazy<DataLocation> = Lazy::new(DataLocation::default);
static PLATFORM_INFO: Lazy<PlatformInfo> = Lazy::new(PlatformInfo::new);
// static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to build HTTP client")
});
const DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("../assets/launcher_profiles.json");

pub struct Storage {
    pub current_instance: Arc<Mutex<instance::Instance>>,
    pub config: Arc<Mutex<Config>>,
}

#[tokio::main]
async fn main() {
    tokio::fs::create_dir_all(&DATA_LOCATION.root)
        .await
        .expect("Could not create appliaction data folder");
    let launcher_profiles_path = DATA_LOCATION.root.join("launcher_profiles.json");
    let _ = tokio::fs::remove_file(&launcher_profiles_path).await;
    tokio::fs::write(&launcher_profiles_path, DEFAULT_LAUNCHER_PROFILE)
        .await
        .expect("Could not create launcher profile");
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
    ";
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
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
            on_frontend_loaded,
            instance::create_instance,
            instance::read_all_instances,
            instance::update_instance,
            instance::delete_instance,
            instance::set_current_instance,
            install::install,
            install::get_minecraft_version_list,
            install::get_fabric_version_list,
            install::get_forge_version_list,
            install::get_quilt_version_list,
            launch::launch,
            config::read_config_file,
            config::update_config,
            config::save_config,
            account::add_microsoft_account,
            account::get_accounts,
            account::refresh_microsoft_account_by_uuid,
            account::refresh_all_microsoft_account,
            account::delete_account,
            account::get_account_by_uuid
        ])
        .manage(Storage {
            current_instance: Arc::new(Mutex::new(instance::Instance::default())),
            config: Arc::new(Mutex::new(config.clone())),
        })
        .append_invoke_initialization_script(init_config_js_script)
        .setup(move |app| {
            print_title();
            match PLATFORM_INFO.os_type {
                OsType::Linux => info!("The program is running on Linux {}", PLATFORM_INFO.version),
                OsType::Osx => info!("The program is running on macOS {}", PLATFORM_INFO.version),
                OsType::Windows => info!(
                    "The program is running on fucking Windows {}",
                    PLATFORM_INFO.version
                ),
            }
            info!(
                "Application data will be saved at: {}",
                DATA_LOCATION.root.display()
            );
            APP_VERSION
                .set(app.package_info().version.to_string())
                .unwrap();
            info!("Main window loaded");
            APP_HANDLE.set(app.app_handle().clone()).unwrap();
            set_hook(Box::new(|info: &PanicHookInfo| {
                APP_HANDLE
                    .get()
                    .unwrap()
                    .dialog()
                    .message(info.to_string())
                    .kind(MessageDialogKind::Error)
                    .title("Fatal Error")
                    .blocking_show();
                let _ = MAIN_WINDOW.close();
                exit(1);
            }));
            Ok(())
        })
        .on_window_event(|window, event| {
            // Do something after app closed
            if window.label() != "main" {
                return;
            };
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.close().unwrap();
                match std::fs::remove_dir_all(&DATA_LOCATION.temp) {
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

#[tauri::command(async)]
async fn on_frontend_loaded(storage: tauri::State<'_, Storage>) -> std::result::Result<(), ()> {
    info!("Frontend loaded");
    let config = &storage.config.lock().unwrap().clone();
    let _ = remind_minecraft_latest(config).await;
    Ok(())
}

async fn remind_minecraft_latest(config: &Config) -> anyhow::Result<()> {
    let (latest, cache_file) = if config.accessibility.snapshot_reminder {
        let latest = VersionManifest::new().await?.latest.snapshot;
        let cache_file = DATA_LOCATION.cache.join("latest_release");
        (latest, cache_file)
    } else if config.accessibility.release_reminder {
        let latest = VersionManifest::new().await?.latest.release;
        let cache_file = DATA_LOCATION.cache.join("latest_snapshot");
        (latest, cache_file)
    } else {
        return Ok(());
    };
    let cache = tokio::fs::read_to_string(&cache_file).await?;
    tokio::fs::write(&cache_file, &latest).await?;
    if latest != cache {
        let _ = MAIN_WINDOW.emit("remind_update", latest);
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
                path: DATA_LOCATION.logs.clone(),
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
    info!("Conic Launcher is starting up");
    info!("Conic Launcher is open source, You can view the source code on Github: https://github.com/conic-apps/launcher");
}
