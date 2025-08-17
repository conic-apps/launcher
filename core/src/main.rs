// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

use folder::DATA_LOCATION;
use log::{error, info};
use tauri::{AppHandle, Manager, Window, WindowEvent, Wry, plugin::TauriPlugin};
#[cfg(debug_assertions)]
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tauri_plugin_log::{Target, TargetKind};

fn main() {
    DATA_LOCATION.init();
    #[cfg(target_os = "linux")]
    {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
    #[allow(clippy::unit_arg)]
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(init_log_builder().build())
        .plugin(single_instance_builder())
        .plugin(config::init())
        .plugin(account::init())
        .plugin(instance::init())
        .plugin(install::init())
        .plugin(launch::init())
        .plugin(folder::init())
        .plugin(platform::init())
        .invoke_handler(tauri::generate_handler![open_path])
        .setup(|_| Ok(print_info()))
        .on_window_event(window_event_handler)
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

fn single_instance_builder() -> TauriPlugin<Wry> {
    tauri_plugin_single_instance::init(|app: &AppHandle, _, _| {
        let windows = app.webview_windows();
        windows
            .values()
            .next()
            .expect("Sorry, no window found")
            .set_focus()
            .expect("Can't Bring Window to Focus");
    })
}

fn window_event_handler(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    };
    if let tauri::WindowEvent::CloseRequested { .. } = event {
        match std::fs::remove_dir_all(&DATA_LOCATION.temp) {
            Ok(_) => info!("Temporary files cleared"),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                error!("Could not clear temp foler")
            }
            _ => (),
        };
        window.close().expect("Could not close window");
    }
}

fn print_info() {
    info!("Conic Launcher is starting up");
    info!(
        "Conic Launcher is open source, You can view the source code on Github: https://github.com/conic-apps/launcher"
    );
    info!("Main window loaded")
}

#[tauri::command]
fn open_path(path: String, with: Option<String>) -> Result<(), tauri_plugin_opener::Error> {
    tauri_plugin_opener::open_path(path, with)
}
