// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{path::PathBuf, result};

use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

/// Errors from Java runtime scanning and launch-time resolution.
///
/// Scanning is best-effort: individual candidates that fail to execute are
/// skipped with a log line rather than aborting the whole scan.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Java version probe timed out after {timeout_secs}s for {path}")]
    TimedOut { path: PathBuf, timeout_secs: u64 },

    #[error("{0}")]
    Scan(String),

    #[error("No suitable Java runtime found")]
    NoSuitableJavaRuntime,

    #[error("No supported Java runtime")]
    NoSupportedJavaRuntime,
}
