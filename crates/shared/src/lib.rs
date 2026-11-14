// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Process-wide helpers shared by the domain crates: the app version, the HTTP
//! client and its proxy setting, the image disk cache, and the `Url` extension
//! the API clients use.

pub mod http_cache;
pub mod notification;
pub mod task;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use log::{info, warn};
use once_cell::sync::{Lazy, OnceCell};
use thiserror::Error;
use url::Url;

/// The version the app was built as. `env!("CARGO_PKG_VERSION")` would read
/// this crate's `0.0.0` placeholder, so the app supplies the real value.
static APP_VERSION: OnceCell<&'static str> = OnceCell::new();

/// Records the app version before the first request, since [`HTTP_CLIENT`]
/// bakes it into its User-Agent when built.
pub fn set_app_version(version: &'static str) {
    if APP_VERSION.set(version).is_err() {
        log::debug!("the app version was already set to {APP_VERSION:?}; ignoring {version:?}");
    }
}

/// The app version, or `"0.0.0"` in a crate's own tests.
pub fn app_version() -> &'static str {
    APP_VERSION.get().copied().unwrap_or_else(|| {
        log::debug!("the app version was never set; the User-Agent will read 0.0.0");
        "0.0.0"
    })
}

/// The output port every domain crate reports through: `Send + Sync + 'static`, so
/// any worker can report a plain snapshot. The app's sink marshals it onto the
/// event loop.
pub type Sink<T> = Arc<dyn Fn(T) + Send + Sync + 'static>;

pub fn silent<T>() -> Sink<T> {
    Arc::new(|_| {})
}

/// A [`Sink`] wrapper that drops repeated values.
///
/// The value must be a plain snapshot: state behind shared handles compares equal
/// to its own past and would drop every update (see
/// `download::progress::DownloadSnapshot`).
///
/// ```
/// # use shared::{ChangeReporter, Sink};
/// let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
/// let sink: Sink<i32> = {
///     let seen = seen.clone();
///     std::sync::Arc::new(move |value| seen.lock().unwrap().push(value))
/// };
/// let reporter = ChangeReporter::new(sink);
/// reporter.report(1);
/// reporter.report(1);
/// reporter.report(2);
/// assert_eq!(*seen.lock().unwrap(), vec![1, 2]);
/// ```
pub struct ChangeReporter<T> {
    sink: Sink<T>,
    last: Mutex<Option<T>>,
}

impl<T: PartialEq + Clone> ChangeReporter<T> {
    pub fn new(sink: Sink<T>) -> Self {
        Self {
            sink,
            last: Mutex::new(None),
        }
    }

    /// Reports `value` unless it equals the last value reported.
    pub fn report(&self, value: T) {
        // A poisoned lock is a panicking sink: nothing useful is left to report,
        // so the event is dropped rather than propagating the panic.
        let Ok(mut last) = self.last.lock() else {
            return;
        };
        if last.as_ref() == Some(&value) {
            return;
        }
        *last = Some(value.clone());
        (self.sink)(value);
    }
}

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
    // The proxy choice and the User-Agent have no other way to be tied to a
    // network failure downstream.
    info!(
        "The shared HTTP client identifies as ConicApps/{}",
        app_version()
    );
    let mut builder = reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(200)
        .user_agent(format!("ConicApps/{}", app_version()))
        .use_rustls_tls();
    if !should_use_system_proxy {
        builder = builder.no_proxy();
    };
    builder
        .build()
        .unwrap_or_else(|error| panic!("Failed to build HTTP client: {error}"))
});

/// Records the config's proxy preference. Must run before the first request
/// (the client above is built on first use and cannot be reconfigured).
pub fn set_system_proxy(use_system_proxy: bool) {
    // Ordering matters here and nothing enforces it: the `Lazy` above reads this
    // cell on first use and cannot be reconfigured afterwards, so a call that
    // lands after the client exists is silently ignored.
    if SHOULD_USE_SYSTEM_PROXY.set(use_system_proxy).is_err() {
        log::warn!(
            "the system proxy preference was set after the HTTP client was built, so \
                    it has no effect"
        );
    }
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
