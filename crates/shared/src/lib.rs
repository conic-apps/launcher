// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use once_cell::sync::{Lazy, OnceCell};
use tauri::{AppHandle, Manager, WebviewWindow};

pub static APP_VERSION: OnceCell<String> = OnceCell::new();
pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
/// use MAIN_WINDOW.emit() to send message to main window
/// TODO: Remove this
pub static MAIN_WINDOW: Lazy<WebviewWindow> = Lazy::new(|| {
    APP_HANDLE
        .get()
        .expect("App handle not set")
        .get_webview_window("main")
        .expect("Could not get main webview window")
});

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .use_rustls_tls()
        .user_agent(format!(
            "ConicApps/{}",
            APP_VERSION
                .get()
                .expect("Could not get app version, so could not set user agent")
        ))
        .build()
        .expect("Failed to build HTTP client")
});
