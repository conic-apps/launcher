// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Discovery, parsing and classification of the Java runtimes installed on the
//! system.
//!
//! The scan runs on the app's own tokio runtime.

pub mod error;
pub mod models;
pub mod mojang;
pub mod parser;
pub mod resolve;
pub mod scanner;

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use once_cell::sync::Lazy;

pub use error::{Error, Result};
pub use models::{
    JavaArch, JavaRuntime, JavaScanResult, JavaVendor, JavaVersionGroup, ScanOptions,
};
pub use resolve::{ResolveJavaOptions, ResolvedJava, resolve_java_executable};
pub use scanner::{scan_java_runtimes, scan_java_runtimes_with};

const SCAN_CACHE_TTL: Duration = Duration::from_secs(30);

static SCAN_CACHE: Lazy<Mutex<Option<(Instant, JavaScanResult)>>> = Lazy::new(|| Mutex::new(None));

/// The settings page rescans every time it is built; the cache keeps that from
/// starting a JVM per candidate on every visit.
pub fn scan_java_runtimes_cached(options: &ScanOptions) -> Result<JavaScanResult> {
    if let Ok(cache) = SCAN_CACHE.lock()
        && let Some((scanned_at, cached)) = cache.as_ref()
        && scanned_at.elapsed() < SCAN_CACHE_TTL
    {
        log::debug!(
            "Reusing the Java scan from {:?} ago ({} runtime(s))",
            scanned_at.elapsed(),
            cached.runtimes.len()
        );
        return Ok(cached.clone());
    }
    log::debug!("Rescanning Java runtimes; the cached scan has expired");

    let result = JavaScanResult::from_runtimes(scan_java_runtimes_with(options)?);
    if let Ok(mut cache) = SCAN_CACHE.lock() {
        *cache = Some((Instant::now(), result.clone()));
    }
    Ok(result)
}
