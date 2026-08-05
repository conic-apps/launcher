// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::time::{SystemTime, UNIX_EPOCH};

use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;

use error::*;

pub mod error;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("statistics")
        .invoke_handler(tauri::generate_handler![
            cmd_get_statistics,
            cmd_get_statistics_by_profile
        ])
        .build()
}

#[command]
async fn cmd_get_statistics() -> Result<Vec<StatisticsEntry>> {
    get_statistics().await
}

#[command]
async fn cmd_get_statistics_by_profile(profile: StatisticsProfile) -> Result<Vec<StatisticsEntry>> {
    get_statistics_by_profile(profile).await
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum StatisticsProfile {
    Microsoft(Uuid),
    Offline(Uuid),
    Yggdrasil(Uuid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatisticsEntry {
    pub profile: StatisticsProfile,
    pub instance_id: Uuid,
    pub launch_at_unix_secs: u64,
}

pub async fn log_launch(profile: StatisticsProfile, instance_id: Uuid) -> Result<()> {
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
    let statistics_path = DATA_LOCATION.root.join("statistics.json");
    let file_content = async_fs::read_to_string(statistics_path).await?;
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
    let path = DATA_LOCATION.root.join("statistics.json");
    async_fs::write(path, serde_json::to_string_pretty(&logs)?).await?;
    Ok(())
}
