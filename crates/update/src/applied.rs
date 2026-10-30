// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The one-shot record of an update that was applied as a session ended.
//!
//! The swap happens after the window closes, so no interface is left to tell
//! the user it happened. [`record`] leaves the target version behind and the
//! next launch [`take`]s it: the process that reads it is the new build, and
//! only it can know the update actually took effect. A record whose version is
//! not the running one is discarded rather than announced, which covers a
//! Windows hand-off whose detached script never completed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::staged::updates_dir;

/// An update that was applied at the end of the previous session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub version: String,
}

impl Applied {
    /// Whether this record names the version that is running now.
    pub fn matches_current(&self) -> bool {
        normalise(&self.version) == normalise(shared::app_version())
    }
}

fn applied_path() -> PathBuf {
    updates_dir().join("applied.json")
}

/// Records `version` as applied, for the next launch to announce.
pub fn record(version: &str) -> Result<()> {
    std::fs::create_dir_all(updates_dir())?;
    let bytes = serde_json::to_vec_pretty(&Applied {
        version: version.to_owned(),
    })?;
    std::fs::write(applied_path(), bytes)?;
    Ok(())
}

/// Reads and removes the record, so an update is announced once.
///
/// The file is removed before it is parsed: a corrupt record would otherwise
/// fail every launch with no way to clear it.
pub fn take() -> Result<Option<Applied>> {
    take_from(&applied_path())
}

fn take_from(path: &Path) -> Result<Option<Applied>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!(
            "could not remove the applied-update record at {}: {error}",
            path.display()
        ),
    }
    Ok(Some(serde_json::from_slice(&bytes)?))
}

/// The server and the bundle may spell a version with a leading `v`; the
/// running app's compiled-in version never does.
fn normalise(version: &str) -> &str {
    version.trim().trim_start_matches(['v', 'V'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_v_does_not_change_the_version() {
        assert_eq!(normalise("v0.2.0"), "0.2.0");
        assert_eq!(normalise("V0.2.0"), "0.2.0");
        assert_eq!(normalise(" 0.2.0 "), "0.2.0");
    }

    #[test]
    fn taking_a_record_removes_it() {
        let dir = std::env::temp_dir().join(format!(
            "conic-applied-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let path = dir.join("applied.json");
        std::fs::write(&path, br#"{"version":"0.2.0"}"#).expect("a record");

        let applied = take_from(&path)
            .expect("the record reads")
            .expect("a record is present");
        assert_eq!(applied.version, "0.2.0");
        assert!(!path.exists());
        assert!(
            take_from(&path)
                .expect("a missing record is fine")
                .is_none()
        );

        std::fs::remove_dir_all(dir).expect("the scratch directory is removable");
    }

    #[test]
    fn a_corrupt_record_is_removed_too() {
        let dir = std::env::temp_dir().join(format!(
            "conic-applied-corrupt-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let path = dir.join("applied.json");
        std::fs::write(&path, b"not json").expect("a corrupt record");

        assert!(take_from(&path).is_err());
        assert!(!path.exists());

        std::fs::remove_dir_all(dir).expect("the scratch directory is removable");
    }
}
