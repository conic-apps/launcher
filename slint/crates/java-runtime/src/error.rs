// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{path::PathBuf, result};

use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

/// Errors of the mirrored Java runtime scanning.
///
/// The variants are the subset of `crates/java-runtime/src/error.rs` that the
/// scan carries. `Aborted` wraps `tauri::Error` and goes away with the Tauri
/// plugin; the launch-time variants (`NoSuitableJavaRuntime`,
/// `NoSupportedJavaRuntime`, both raised by `resolve.rs`) arrive with the
/// launch view.
///
/// Scanning is best-effort either way: individual candidates that fail to
/// execute are skipped with a log line rather than aborting the whole scan.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Java version probe timed out after {timeout_secs}s for {path}")]
    TimedOut { path: PathBuf, timeout_secs: u64 },

    #[error("{0}")]
    Scan(String),
}
