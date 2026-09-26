// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free stand-in for the two items `crates/account` takes from
//! `crates/shared`: the async HTTP client and the [`UrlExt`] path builder.
//!
//! `slint-install` inlines the same client in its own `lib.rs` ("The shared
//! HTTP client, standing in for `shared::HTTP_CLIENT`"), for the same reason:
//! the original's `HTTP_CLIENT` forces rustls, which is unnecessary here as the
//! workspace enables no other TLS backend, and a crate of its own for forty
//! lines is not worth the extra hop. The builder, the pool settings, the user
//! agent and the proxy switch are otherwise identical, so the two crates make
//! the same requests in the same way.

use std::time::Duration;

use log::{info, warn};
use once_cell::sync::{Lazy, OnceCell};
use thiserror::Error;
use url::Url;

/// See `set_system_proxy`.
pub static SHOULD_USE_SYSTEM_PROXY: OnceCell<bool> = OnceCell::new();

/// Records the config's proxy preference. Must run before the first request
/// (the client below is built on first use and cannot be reconfigured).
pub fn set_system_proxy(use_system_proxy: bool) {
    let _ = SHOULD_USE_SYSTEM_PROXY.set(use_system_proxy);
}

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
    if should_use_system_proxy {
        info!("Using system proxy")
    } else {
        info!("Shouldn't use system proxy")
    }
    let mut builder = reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(200)
        .user_agent(format!("ConicApps/{}", env!("CARGO_PKG_VERSION")));
    if !should_use_system_proxy {
        builder = builder.no_proxy();
    };
    builder.build().expect("Failed to build HTTP client")
});

#[derive(Debug, Error)]
pub enum UrlExtError {
    #[error("URL cannot be used as a base URL")]
    InvalidBaseUrl,
}

/// Appends path segments to a URL, as `shared::UrlExt` does.
pub trait UrlExt {
    fn append_path<I, S>(self, segments: I) -> Result<Self, UrlExtError>
    where
        Self: Sized,
        I: IntoIterator<Item = S>,
        S: AsRef<str>;
}

impl UrlExt for Url {
    fn append_path<I, S>(mut self, segments: I) -> Result<Self, UrlExtError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        {
            let mut path_segments = self
                .path_segments_mut()
                .map_err(|_| UrlExtError::InvalidBaseUrl)?;

            for segment in segments {
                path_segments.push(segment.as_ref());
            }
        }

        Ok(self)
    }
}
