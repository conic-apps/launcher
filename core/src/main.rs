// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

use config::{Config, load_config_file};
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
    let config = load_config_file().unwrap_or_else(|e| {
        log::error!("FATAL: Unable to load or reset config file!");
        panic!("{e}")
    });
    info!("Conic Launcher is starting up");
    info!(
        "Conic Launcher is open source, You can view the source code on Github: https://github.com/conic-apps/launcher"
    );
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
        .append_invoke_initialization_script(get_init_config_script(&config))
        .setup(|_| Ok(info!("Main window loaded")))
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

fn get_init_config_script(config: &Config) -> String {
    "
        Object.defineProperty(window, '__APPLICATION_CONFIG__', {
            value: JSON.parse(`"
        .to_string()
        + serde_json::to_string_pretty(config)
            .expect("The program is broken")
            .as_ref()
        + "`)
        })
    "
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
