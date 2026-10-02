// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Process-wide helpers: the shared crate's version constant (`APP_VERSION`),
//! the shared HTTP client and its proxy preference, and the `Url` extension the
//! API clients use.
//!
//! The crate exists on its own — rather than as a module inlined into each
//! consumer — so every crate that makes requests shares one client: `account`,
//! `curseforge`, `download`, `install` and `modrinth` all take [`HTTP_CLIENT`]
//! from here, and the app sets the proxy preference once through
//! [`set_system_proxy`].

use std::time::Duration;

use log::{info, warn};
use once_cell::sync::{Lazy, OnceCell};
use thiserror::Error;
use url::Url;

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
    if should_use_system_proxy {
        info!("Using system proxy")
    } else {
        info!("Shouldn't use system proxy")
    }
    let mut builder = reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(200)
        .user_agent(format!("ConicApps/{}", APP_VERSION))
        .use_rustls_tls();
    if !should_use_system_proxy {
        builder = builder.no_proxy();
    };
    builder.build().expect("Failed to build HTTP client")
});

/// Records the config's proxy preference. Must run before the first request
/// (the client above is built on first use and cannot be reconfigured).
pub fn set_system_proxy(use_system_proxy: bool) {
    let _ = SHOULD_USE_SYSTEM_PROXY.set(use_system_proxy);
}

#[derive(Debug, Error)]
pub enum UrlExtError {
    #[error("URL cannot be used as a base URL")]
    InvalidBaseUrl,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn append(base: &str, segments: &[&str]) -> String {
        Url::parse(base)
            .expect("valid base")
            .append_path(segments.iter().copied())
            .expect("can be a base")
            .to_string()
    }

    #[test]
    fn segments_are_appended_below_the_existing_path() {
        assert_eq!(
            append("https://example.com/api", &["v2", "project"]),
            "https://example.com/api/v2/project"
        );
    }

    #[test]
    fn a_segment_needing_encoding_is_encoded() {
        assert_eq!(
            append("https://example.com", &["hello world"]),
            "https://example.com/hello%20world"
        );
    }

    #[test]
    fn appending_nothing_leaves_the_url_alone() {
        assert_eq!(
            append("https://example.com/api", &[]),
            "https://example.com/api"
        );
    }

    #[test]
    fn a_url_that_cannot_be_a_base_is_rejected() {
        let result = Url::parse("mailto:someone@example.com")
            .unwrap()
            .append_path(["x"]);
        assert!(matches!(result, Err(UrlExtError::InvalidBaseUrl)));
    }
}
