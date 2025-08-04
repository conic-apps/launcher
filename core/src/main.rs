// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// #![deny(clippy::unwrap_used)]

use std::panic::{PanicHookInfo, set_hook};

use backtrace::Backtrace;
use config::{Config, load_config_file};
use folder::DATA_LOCATION;
use log::{debug, error, info};
use platform::PLATFORM_INFO;
use shared::{APP_HANDLE, MAIN_WINDOW};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
#[cfg(debug_assertions)]
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tauri_plugin_log::{Target, TargetKind};
use version::VersionManifest;

#[tokio::main]
async fn main() {
    DATA_LOCATION
        .init()
        .await
        .expect("Could not init data folder");
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
        .append_invoke_initialization_script(init_config_js_script)
        .setup(move |app| {
            print_title();
            std::fs::write(
                DATA_LOCATION.root.join("platform.json"),
                serde_json::to_string_pretty(&PLATFORM_INFO.clone()).unwrap(),
            )
            .unwrap();
            info!("Main window loaded");
            APP_HANDLE.set(app.app_handle().clone()).unwrap();
            set_hook(Box::new(|info: &PanicHookInfo| {
                let backtrace = format!("{:#?}", Backtrace::new());
                let backtrace_first_ten_lines: Vec<&str> = backtrace.lines().take(12).collect();
                println!("{backtrace:#?}");
                APP_HANDLE
                    .get()
                    .unwrap()
                    .dialog()
                    .message(format!(
                        "{}\nBacktrace:\n{}\nmore {} lines not shown...",
                        info,
                        backtrace_first_ten_lines.join("\n"),
                        backtrace.lines().count() - 12
                    ))
                    .kind(MessageDialogKind::Error)
                    .title("Fatal Error")
                    .blocking_show();
                let _ = MAIN_WINDOW.close();
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

#[tauri::command]
async fn _on_frontend_loaded(config: Config) -> std::result::Result<(), ()> {
    info!("Frontend loaded");
    let _ = _remind_minecraft_latest(&config).await;
    Ok(())
}

async fn _remind_minecraft_latest(config: &Config) -> anyhow::Result<()> {
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
    info!(
        "Conic Launcher is open source, You can view the source code on Github: https://github.com/conic-apps/launcher"
    );
}
