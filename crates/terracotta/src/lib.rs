// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use download::progress::DownloadState;
use folder::DATA_LOCATION;
use futures::stream::{AbortHandle, Abortable};
use tauri::{
    Manager, Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};

use crate::library::load_library_from_file;
use crate::{
    error::Result,
    library::{check_library_valid, download_library},
};
use sha2::Digest;

pub mod library;
mod metadata;

#[derive(Clone, Default)]
struct PluginState {
    abort_handle: Arc<Mutex<Option<AbortHandle>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("terracotta")
        .invoke_handler(tauri::generate_handler![
            cmd_spawn_download_library_task,
            cmd_cancel_download_library_task,
            cmd_check_library_valid
        ])
        .setup(|app, _| {
            app.manage(PluginState::default());
            Ok(())
        })
        .build()
}

#[command]
async fn cmd_spawn_download_library_task(
    state: State<'_, PluginState>,
    channel: Channel<DownloadState>,
) -> Result<()> {
    let progress = DownloadState::default();
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
                std::thread::sleep(Duration::from_millis(50));
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
async fn cmd_check_library_valid() -> Result<()> {
    check_library_valid().await
}
