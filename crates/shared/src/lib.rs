// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use log::warn;
use once_cell::sync::{Lazy, OnceCell};

pub static APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static SHOULD_USE_SYSTEM_PROXY: OnceCell<bool> = OnceCell::new();

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let should_use_system_proxy = match SHOULD_USE_SYSTEM_PROXY.get() {
        Some(should_use_system_proxy) => should_use_system_proxy.to_owned(),
        None => {
            warn!(
                "Unable to determine whether to use the system proxy; using the default settings."
            );
            true
        }
    };
    let mut builder = reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(200)
        .use_rustls_tls()
        .user_agent(format!("ConicApps/{}", APP_VERSION));
    if !should_use_system_proxy {
        builder = builder.no_proxy();
    };
    builder.build().expect("Failed to build HTTP client")
});
