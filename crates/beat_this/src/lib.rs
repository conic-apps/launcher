// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use download::task::Progress;
use folder::DATA_LOCATION;
use futures::stream::{AbortHandle, Abortable};
use tauri::{
    Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};

use crate::{
    error::Result,
    library::{download_library, ensure_library},
};
use crate::{ffi::BeatAnalysis, library::load_library_from_file};
use sha2::Digest;

pub mod error;
pub mod ffi;
pub mod library;
mod metadata;

#[derive(Clone, Default)]
struct PluginState {
    abort_handle: Arc<Mutex<Option<AbortHandle>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("beat")
        .invoke_handler(tauri::generate_handler![
            cmd_spawn_download_library_task,
            cmd_cancel_download_library_task,
            cmd_parse_audio_file
        ])
        .build()
}

#[command]
async fn cmd_spawn_download_library_task(
    state: State<'_, PluginState>,
    channel: Channel<Progress>,
) -> Result<()> {
    let progress = Progress::default();
    let (handle, reg) = AbortHandle::new_pair();
    let future = Abortable::new(download_library(&progress), reg);
    {
        let mut current_task = state.abort_handle.lock().expect("Internal error");
        *current_task = Some(handle);
    }
    let finished = Arc::new(AtomicBool::new(false));
    let event_sender_thread = {
        let progress_cloned = progress.clone();
        let finished = finished.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                let _ = channel.send(progress_cloned.clone());
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    let result = match future.await {
        Ok(result) => result,
        Err(e) => Err(crate::error::Error::Aborted(e)),
    };
    finished.store(true, Ordering::SeqCst);
    let _ = event_sender_thread.join();
    result
}

#[command]
fn cmd_cancel_download_library_task(state: State<'_, PluginState>) {
    let mut current_task = state.abort_handle.lock().expect("Internal error");
    if let Some(handle) = current_task.clone() {
        handle.abort();
    }
    *current_task = None;
}

#[command]
async fn cmd_parse_audio_file(path: String) -> Result<BeatAnalysis> {
    ensure_library().await?;
    parse_audio_file(path).await
}

pub async fn parse_audio_file(path: String) -> Result<BeatAnalysis> {
    let mut sha256_hasher = sha2::Sha256::new();
    let file_content = async_fs::read(&path).await?;
    sha256_hasher.update(&file_content);
    let sha256 = format!("{:x}", sha256_hasher.finalize());

    let cache_dir = DATA_LOCATION.cache.join("beat_this");
    let cache_path = cache_dir.join(format!("{sha256}.json"));

    if let Ok(cached) = async_fs::read_to_string(&cache_path).await
        && let Ok(analysis) = serde_json::from_str::<BeatAnalysis>(&cached)
    {
        return Ok(analysis);
    }

    let analysis = unsafe {
        let library = load_library_from_file(
            DATA_LOCATION
                .runtime
                .join("native")
                .join(metadata::LIBRARY.filename),
        )
        .await?;
        ffi::parse_audio_file(library, path)
    };

    if let Ok(ref a) = analysis
        && let Ok(json) = serde_json::to_string(a)
    {
        let _ = async_fs::create_dir_all(&cache_dir).await;
        let _ = async_fs::write(&cache_path, json).await;
    }

    analysis
}
