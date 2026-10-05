// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Process-wide helpers: the shared crate's version constant (`APP_VERSION`),
//! the shared HTTP client and its proxy preference, a disk cache for the images
//! the UI fetches, and the `Url` extension the API clients use.
//!
//! The crate exists on its own — rather than as a module inlined into each
//! consumer — so every crate that makes requests shares one client: `account`,
//! `curseforge`, `download`, `install` and `modrinth` all take [`HTTP_CLIENT`]
//! from here, and the app sets the proxy preference once through
//! [`set_system_proxy`].
//!
//! [`http_cache`] sits beside the client rather than inside it because it caches
//! something the client knows nothing about: those requests want a stored copy,
//! a second one behind the same URL refused, and never a peer on the user's own
//! network. The client is left as the plain transport the API crates expect.

pub mod http_cache;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use log::{info, warn};
use once_cell::sync::{Lazy, OnceCell};
use thiserror::Error;
use url::Url;

pub static APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The callback through which a crate reports a task's state to whoever owns
/// the interface.
///
/// This is the one shape every inner crate uses for an output port. It is
/// `Send + Sync` so the use case may report from any worker or task and clone
/// the handle into several of them, and `'static` because the port outlives any
/// one call. The crate names a type alias over it for its own event, so the
/// vocabulary (`InstallSink`, `LoginSink`, …) is the crate's and the mechanism
/// is not.
///
/// The sink must not do the work: a use case reports a *plain snapshot* and
/// returns, and the sink (in the app) marshals it onto the event loop. A crate
/// that called `upgrade_in_event_loop` itself would depend on Slint, which is
/// exactly the dependency the port exists to remove.
pub type Sink<T> = Arc<dyn Fn(T) + Send + Sync + 'static>;

/// A sink that drops every event, for a caller that has no interface to report
/// to (a headless run, a test).
pub fn silent<T>() -> Sink<T> {
    Arc::new(|_| {})
}

/// A [`Sink`] wrapper that drops repeated values.
///
/// The counter half of progress is sampled on a clock, so the same value is
/// read many times while nothing moves; only a change is worth crossing a
/// thread boundary and waking the event loop for. `PartialEq` is the change
/// test, which is why the value has to be a plain snapshot and not a handle: a
/// struct whose state sits behind `Arc`s compares equal to its own past, which
/// would drop every update (see `download::progress::DownloadSnapshot`).
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
