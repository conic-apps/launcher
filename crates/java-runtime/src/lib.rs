// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Discovery, parsing and classification of installed Java runtimes.
//!
//! The overall design is inspired by HMCL's Java toolchain management
//! (https://github.com/HMCL-dev/HMCL):
//!
//! - well-known search paths per platform, plus `JAVA_HOME`, `PATH`, Windows
//!   registry keys, Minecraft's bundled runtimes and launcher-managed homes
//!   (see [`scanner`]);
//! - metadata from the JDK `release` file merged with the output of
//!   `java -XshowSettings:properties -version` (see [`parser`]);
//! - deduplication by canonical executable path and classification by Java
//!   major version and normalized vendor, so the UI never has to parse version
//!   strings (see [`models`]).
//!
//! The crate also ships a thin Tauri plugin ([`init`]) exposing a cached
//! [`cmd_scan_java`] command to the frontend. All heavy work runs on a blocking
//! thread pool via `spawn_blocking` so the async runtime (and therefore the UI)
//! is never blocked, and scan results are cached briefly so repeated frontend
//! refreshes do not re-scan the system.

pub mod error;
pub mod models;
pub mod parser;
pub mod scanner;

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use folder::DATA_LOCATION;
use tauri::{
    Manager, Runtime, State, command,
    plugin::{Builder, TauriPlugin},
};

pub use error::{Error, Result};
pub use models::{
    JavaArch, JavaRuntime, JavaScanResult, JavaVendor, JavaVersionGroup, ScanOptions,
};
pub use scanner::{scan_java_runtimes, scan_java_runtimes_with};

/// How long a scan result is reused before the next `cmd_scan_java` rescans.
const SCAN_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Default)]
struct ScanState {
    cache: Mutex<Option<(Instant, JavaScanResult)>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("java-runtime")
        .setup(|app, _| {
            app.manage(ScanState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![cmd_scan_java])
        .build()
}

/// Scans the system for installed Java runtimes.
///
/// The scan runs on a background thread (`spawn_blocking`) so it never blocks
/// the async runtime. Launcher-managed runtimes under `DATA_LOCATION.runtime`
/// are scanned and flagged as `is_managed`.
#[command]
async fn cmd_scan_java(state: State<'_, ScanState>) -> Result<JavaScanResult> {
    if let Some((cached_at, cached)) = state.cache.lock().expect("Internal error").as_ref()
        && cached_at.elapsed() < SCAN_CACHE_TTL
    {
        return Ok(cached.clone());
    }

    let options = ScanOptions {
        extra_home_dirs: Vec::new(),
        managed_dirs: vec![DATA_LOCATION.runtime.clone()],
    };
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<JavaScanResult> {
        Ok(JavaScanResult::from_runtimes(scan_java_runtimes_with(
            &options,
        )?))
    })
    .await??;

    *state.cache.lock().expect("Internal error") = Some((Instant::now(), result.clone()));
    Ok(result)
}
