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
    let mut entries = get_statistics().await.unwrap_or_default();
    entries.push(entry);
    save_logs_file(entries).await?;
    Ok(())
}

pub async fn get_statistics() -> Result<Vec<StatisticsEntry>> {
    let statistics_path = LOCATIONS.launcher.root.join("statistics.json");
    let file_content = tokio::fs::read_to_string(statistics_path).await?;
    Ok(serde_json::from_str(&file_content)?)
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
    tokio::fs::write(path, serde_json::to_string_pretty(&logs)?).await?;
    Ok(())
}
