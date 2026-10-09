// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The per-launch log the launcher writes to `statistics.json`.
//!
//! [`log_launch`] appends one entry per launch; the two getters read the file
//! back, either all of it or filtered to one profile.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use storage::LOCATIONS;
use uuid::Uuid;

use error::*;

pub mod error;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum StatisticsProfile {
    Microsoft(Uuid),
    Offline(Uuid),
    Yggdrasil(Uuid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatisticsEntry {
    pub profile: StatisticsProfile,
    pub instance_id: String,
    pub launch_at_unix_secs: u64,
}

pub async fn log_launch(profile: StatisticsProfile, instance_id: String) -> Result<()> {
    let launch_at_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Incorrect system time")
        .as_secs();
    let entry = StatisticsEntry {
        profile,
        instance_id,
        launch_at_unix_secs,
    };
    // A failure to read the existing entries is dropped, and then the file is
    // *overwritten* with just this one — so a corrupt `statistics.json` silently
    // discards the whole history. Worth a line of its own.
    let mut entries = get_statistics().await.unwrap_or_else(|error| {
        log::warn!(
            "The statistics history could not be read ({error}); only this launch will be \
             recorded"
        );
        Vec::new()
    });
    entries.push(entry);
    save_logs_file(entries).await?;
    Ok(())
}

pub async fn get_statistics() -> Result<Vec<StatisticsEntry>> {
    let statistics_path = LOCATIONS.launcher.root.join("statistics.json");
    // A missing file on a fresh install is ordinary and stays quiet; a file that
    // is there and does not parse means the contribution graph is silently empty
    // from now on, since every write after this point is based on what is read.
    let file_content = tokio::fs::read_to_string(&statistics_path).await?;
    match serde_json::from_str(&file_content) {
        Ok(entries) => Ok(entries),
        Err(error) => {
            log::warn!(
                "{} is not valid statistics json: {error}",
                statistics_path.display()
            );
            Err(error.into())
        }
    }
}

pub async fn get_statistics_by_profile(profile: StatisticsProfile) -> Result<Vec<StatisticsEntry>> {
    Ok(get_statistics()
        .await?
        .into_iter()
        .filter(|x| x.profile == profile)
        .collect())
}

async fn save_logs_file(logs: Vec<StatisticsEntry>) -> Result<()> {
    let path = LOCATIONS.launcher.root.join("statistics.json");
    // The caller aborts on this error, so it is logged here as well: a panic
    // under `panic = "abort"` is not something a user can report usefully.
    let entry_count = logs.len();
    tokio::fs::write(&path, serde_json::to_string_pretty(&logs)?)
        .await
        .map_err(|error| {
            log::error!(
                "the {entry_count} recorded launch(es) could not be written to {}: {error}",
                path.display()
            );
            error
        })?;
    Ok(())
}
