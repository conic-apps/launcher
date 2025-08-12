// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// #![deny(clippy::unwrap_used)]

use config::load_config_file;
use folder::DATA_LOCATION;
use log::{error, info};
use platform::PLATFORM_INFO;
use shared::APP_VERSION;
use tauri::{AppHandle, Manager};
#[cfg(debug_assertions)]
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tauri_plugin_log::{Target, TargetKind};

fn main() {
    APP_VERSION
        .set(env!("CARGO_PKG_VERSION").to_string())
        .unwrap();
    DATA_LOCATION.init().expect("Could not init data folder");
    #[cfg(target_os = "linux")]
    {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
    let config = load_config_file();
    let init_config_js_script = "
        Object.defineProperty(window, '__APPLICATION_CONFIG__', {
            value: JSON.parse(`"
        .to_string()
        + serde_json::to_string_pretty(&config).unwrap().as_ref()
        + "`)
        })
    ";
    info!("Conic Launcher is starting up");
    info!(
        "Conic Launcher is open source, You can view the source code on Github: https://github.com/conic-apps/launcher"
    );
    let single_instance_closure = |app: &AppHandle, _, _| {
        let windows = app.webview_windows();
        windows
            .values()
            .next()
            .expect("Sorry, no window found")
            .set_focus()
            .expect("Can't Bring Window to Focus");
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(init_log_builder().build())
        .plugin(tauri_plugin_single_instance::init(single_instance_closure))
        .plugin(config::init())
        .plugin(account::init())
        .plugin(instance::init())
        .plugin(install::init())
        .plugin(launch::init())
        .plugin(platform::init())
        .append_invoke_initialization_script(init_config_js_script)
        .setup(|_app| {
            std::fs::write(
                DATA_LOCATION.root.join("platform.json"),
                serde_json::to_string_pretty(&PLATFORM_INFO.clone()).unwrap(),
            )
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
