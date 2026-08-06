// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{path::PathBuf, result};

use serde::Serialize;
use serde_with::serde_as;
use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

/// Errors that can occur while scanning for Java runtimes.
///
/// Scanning is best-effort: individual candidates that fail to execute are
/// skipped with a log line rather than aborting the whole scan. These errors
/// are surfaced to the frontend by the Tauri commands, so they are serialized
/// with a `kind` tag and a human-readable `message`.
#[serde_as]
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum Error {
    #[error(transparent)]
    Io(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::io::Error,
    ),

    #[error("Java version probe timed out after {timeout_secs}s for {path}")]
    TimedOut { path: PathBuf, timeout_secs: u64 },

    #[error("{0}")]
    Scan(String),

    #[error(transparent)]
    Aborted(
        #[from]
        #[serde_as(as = "serde_with::DisplayFromStr")]
        tauri::Error,
    ),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_tagged_message() {
        let json = serde_json::to_string(&Error::Scan("boom".to_owned())).unwrap();
        assert_eq!(json, r#"{"kind":"Scan","message":"boom"}"#);

        let json = serde_json::to_string(&Error::Io(std::io::Error::from(std::io::ErrorKind::NotFound)))
            .unwrap();
        assert_eq!(
            json,
            r#"{"kind":"Io","message":"entity not found"}"#
        );
    }
}
