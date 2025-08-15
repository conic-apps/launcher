// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use once_cell::sync::Lazy;

pub static APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .use_rustls_tls()
        .user_agent(format!("ConicApps/{}", APP_VERSION))
        .build()
        .expect("Failed to build HTTP client")
});
