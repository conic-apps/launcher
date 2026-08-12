// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Launcher self-update utilities.
//!
//! Wraps `tauri-plugin-updater` and targets the ConicMC update server, which
//! exposes one endpoint per channel (`stable` / `beta` / `nightly`):
//!
//! `GET /v1/{channel}/{target}/{arch}/{current_version}`

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use config::UpdateChannel;
use log::info;
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

pub mod error;

pub use error::*;

/// Default update server. Can be overridden at runtime for development with
/// the `CONIC_UPDATE_BASE_URL` environment variable.
const UPDATE_SERVER_BASE_URL: &str = "https://brkdr.dpdns.org";

#[derive(Clone, Default)]
struct PluginState {
    task: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("update")
        .invoke_handler(tauri::generate_handler![
            cmd_check_update,
            cmd_download_and_install_update,
            cmd_cancel_update
        ])
        .setup(|app, _| {
            app.manage(PluginState::default());
            Ok(())
        })
        .build()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub date: Option<i64>,
    pub notes: Option<String>,
    pub download_url: String,
}

impl From<Update> for UpdateInfo {
    fn from(update: Update) -> Self {
        Self {
            version: update.version,
            date: update.date.map(|date| date.unix_timestamp()),
            notes: update.body,
            download_url: update.download_url.to_string(),
        }
    }
}

/// Progress events emitted while checking / downloading / installing an update.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "phase")]
pub enum UpdateProgress {
    Checking,
    Downloading { downloaded: u64, total: Option<u64> },
    Downloaded,
    Installing,
}

fn build_updater<R: Runtime>(
    app: &AppHandle<R>,
    channel: &UpdateChannel,
) -> Result<tauri_plugin_updater::UpdaterBuilder> {
    let base_url = std::env::var("CONIC_UPDATE_BASE_URL")
        .unwrap_or_else(|_| UPDATE_SERVER_BASE_URL.to_string());
    let endpoint = Url::parse(&format!(
        "{}/v1/{}/{{{{target}}}}/{{{{arch}}}}/{{{{current_version}}}}",
        base_url.trim_end_matches('/'),
        channel.as_str(),
    ))?;
    Ok(app
        .updater_builder()
        .endpoints(vec![endpoint])?
        .version_comparator(|current, remote| current != remote.version))
}

/// Checks the given channel for an update to the running launcher.
#[command]
async fn cmd_check_update<R: Runtime>(
    app: AppHandle<R>,
    channel: UpdateChannel,
) -> Result<Option<UpdateInfo>> {
    let update = match build_updater(&app, &channel)?.build()?.check().await? {
        Some(update) => update,
        None => return Ok(None),
    };
    Ok(Some(update.into()))
}

/// Downloads and installs the newest update of the given channel.
///
/// Progress is streamed over `on_progress`. On success the application is
/// restarted by the updater plugin.
#[command]
async fn cmd_download_and_install_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    channel: UpdateChannel,
    on_progress: Channel<UpdateProgress>,
) -> Result<()> {
    let on_progress = on_progress.clone();
    let handle = tokio::spawn(async move {
        let result = download_and_install(app, channel, &on_progress).await;
        info!("Update task finished: {:?}", result.as_ref().map(|_| ()));
        result
    });
    {
        let mut current_task = state.task.lock().expect("Internal error");
        *current_task = Some(handle.abort_handle());
    }
    let result: Result<()> = handle.await.map_err(Error::Join)?;
    {
        let mut current_task = state.task.lock().expect("Internal error");
        *current_task = None;
    }
    result
}

#[command]
fn cmd_cancel_update(state: State<'_, PluginState>) {
    if let Some(handle) = state.task.lock().expect("Internal error").take() {
        handle.abort();
        info!("Update cancelled");
    }
}

async fn download_and_install<R: Runtime>(
    app: AppHandle<R>,
    channel: UpdateChannel,
    on_progress: &Channel<UpdateProgress>,
) -> Result<()> {
    let _ = on_progress.send(UpdateProgress::Checking);
    let update = match build_updater(&app, &channel)?.build()?.check().await? {
        Some(update) => update,
        None => return Err(Error::NoUpdateAvailable),
    };
    info!("Downloading update {}", update.version);

    let downloaded = Arc::new(AtomicU64::new(0));
    let last_event = Arc::new(Mutex::new(Instant::now()));
    let _ = on_progress.send(UpdateProgress::Downloading {
        downloaded: 0,
        total: None,
    });
    let (on_progress_cloned, downloaded_cloned, last_event_cloned) =
        (on_progress.clone(), downloaded.clone(), last_event.clone());
    update
        .download_and_install(
            move |chunk_length: usize, content_length: Option<u64>| {
                let downloaded = downloaded_cloned.fetch_add(chunk_length as u64, Ordering::SeqCst)
                    + chunk_length as u64;
                let mut last = last_event_cloned.lock().expect("Internal error");
                if last.elapsed() < Duration::from_millis(100) {
                    return;
                }
                *last = Instant::now();
                let _ = on_progress_cloned.send(UpdateProgress::Downloading {
                    downloaded,
                    total: content_length,
                });
            },
            || {
                let _ = on_progress.send(UpdateProgress::Downloaded);
            },
        )
        .await?;

    info!("Update downloaded, installing");
    let _ = on_progress.send(UpdateProgress::Installing);
    Ok(())
}
