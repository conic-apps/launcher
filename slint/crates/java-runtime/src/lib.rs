// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/java-runtime`: discovery, parsing and
//! classification of the Java runtimes installed on the system.
//!
//! The original crate exposes these through an `async` Tauri command that
//! caches its answer in the plugin state. This mirror keeps the same shape:
//!
//!   * [`scan_java_runtimes`] / [`scan_java_runtimes_with`] are the blocking
//!     functions the original's command runs on `spawn_blocking`;
//!   * [`scan_java_runtimes_cached`] stands in for the command itself
//!     (`cmd_scan_java`) and owns the cache the Tauri plugin keeps in its
//!     `PluginState`, down to the same 30 s TTL;
//!   * it carries the scanning half of the crate only. `resolve.rs` (which
//!     runtime to launch the game with) and `mojang.rs` (the launcher-managed
//!     runtimes it downloads) arrive with the launch view — see
//!     `slint/README.md`.
//!
//! Structures, search paths and sorting match `crates/java-runtime/src/*.rs` so
//! the two crates can be diffed against each other. The one deliberate
//! difference is the `serde` derives: the original's exist for the Tauri IPC
//! boundary, which the Slint app does not have, so the UI calls
//! [`JavaVendor::display_name`] / [`JavaArch::display_name`] instead.

pub mod error;
pub mod models;
pub mod parser;
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
pub use scanner::{scan_java_runtimes, scan_java_runtimes_with};

/// How long a scan result is reused before the next scan, mirroring
/// `SCAN_CACHE_TTL` in `crates/java-runtime/src/lib.rs`.
const SCAN_CACHE_TTL: Duration = Duration::from_secs(30);

/// The last scan, standing in for the `ScanState` the Tauri plugin manages.
static SCAN_CACHE: Lazy<Mutex<Option<(Instant, JavaScanResult)>>> = Lazy::new(|| Mutex::new(None));

/// Scans the system for installed Java runtimes, reusing the previous result
/// while it is younger than [`SCAN_CACHE_TTL`] (`cmd_scan_java`).
///
/// The settings page rescans every time it is built, so the cache is what keeps
/// that from starting a JVM per candidate on every visit, as in the original.
pub fn scan_java_runtimes_cached(options: &ScanOptions) -> Result<JavaScanResult> {
    if let Ok(cache) = SCAN_CACHE.lock()
        && let Some((scanned_at, cached)) = cache.as_ref()
        && scanned_at.elapsed() < SCAN_CACHE_TTL
    {
        return Ok(cached.clone());
    }

    let result = JavaScanResult::from_runtimes(scan_java_runtimes_with(options)?);
    if let Ok(mut cache) = SCAN_CACHE.lock() {
        *cache = Some((Instant::now(), result.clone()));
    }
    Ok(result)
}
