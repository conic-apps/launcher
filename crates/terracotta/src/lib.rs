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

use crate::{
    error::{Error, Result},
    ffi::{
        Terracotta, TerracottaConfig, TerracottaEvent, TerracottaState, terracotta_from_library,
    },
    library::{check_library_valid, download_library, load_library_from_file},
    metadata::LIBRARY,
};

pub mod error;
pub mod ffi;
pub mod library;
mod metadata;

#[derive(Clone, Default)]
struct PluginState {
    abort_handle: Arc<Mutex<Option<AbortHandle>>>,
    terracotta: Arc<Mutex<Option<Arc<Terracotta>>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("terracotta")
        .invoke_handler(tauri::generate_handler![
            cmd_spawn_download_library_task,
            cmd_cancel_download_library_task,
            cmd_check_library_valid,
            cmd_ensure_library,
            cmd_create_context,
            cmd_destroy_context,
            cmd_create_room,
            cmd_join_room,
            cmd_set_waiting,
            cmd_get_state,
            cmd_poll_event,
            cmd_verify_room_code,
            cmd_version,
        ])
        .setup(|app, _| {
            app.manage(PluginState::default());
            Ok(())
        })
        .on_drop(|app| {
            if let Some(state) = app.try_state::<PluginState>()
                && let Some(terracotta) = state.terracotta.lock().expect("Internal error").clone()
            {
                terracotta.destroy_context();
            }
        })
        .build()
}

fn get_terracotta(state: &State<'_, PluginState>) -> Result<Arc<Terracotta>> {
    state
        .terracotta
        .lock()
        .expect("Internal error")
        .clone()
        .ok_or(Error::NotLoaded)
}

fn library_path() -> std::path::PathBuf {
    DATA_LOCATION
        .runtime
        .join("terracotta")
        .join(LIBRARY.filename)
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

/// Loads the terracotta library exactly once. The library stays loaded for the
/// whole launcher run; only the context handle is created/destroyed per session.
#[command]
async fn cmd_ensure_library(state: State<'_, PluginState>) -> Result<()> {
    if state.terracotta.lock().expect("Internal error").is_some() {
        return Ok(());
    }
    let library = unsafe { load_library_from_file(library_path()).await? };
    let terracotta = unsafe { terracotta_from_library(library)? };
    let mut slot = state.terracotta.lock().expect("Internal error");
    if slot.is_none() {
        *slot = Some(Arc::new(terracotta));
    }
    Ok(())
}

#[command]
async fn cmd_create_context(state: State<'_, PluginState>, config: TerracottaConfig) -> Result<()> {
    let terracotta = get_terracotta(&state)?;
    tauri::async_runtime::spawn_blocking(move || terracotta.create_context(&config))
        .await
        .expect("Internal error")
}

#[command]
fn cmd_destroy_context(state: State<'_, PluginState>) {
    if let Ok(terracotta) = get_terracotta(&state) {
        terracotta.destroy_context();
    }
}

#[command]
fn cmd_create_room(
    state: State<'_, PluginState>,
    player_name: Option<String>,
    room_code: Option<String>,
) -> Result<()> {
    get_terracotta(&state)?.create_room(player_name.as_deref(), room_code.as_deref())
}

#[command]
fn cmd_join_room(
    state: State<'_, PluginState>,
    room_code: String,
    player_name: Option<String>,
) -> Result<()> {
    get_terracotta(&state)?.join_room(&room_code, player_name.as_deref())
}

#[command]
fn cmd_set_waiting(state: State<'_, PluginState>) -> Result<()> {
    get_terracotta(&state)?.set_waiting()
}

#[command]
fn cmd_get_state(state: State<'_, PluginState>) -> Result<TerracottaState> {
    get_terracotta(&state)?.get_state()
}

#[command]
fn cmd_poll_event(state: State<'_, PluginState>) -> Result<Option<TerracottaEvent>> {
    get_terracotta(&state)?.poll_event()
}

#[command]
fn cmd_verify_room_code(state: State<'_, PluginState>, room_code: String) -> Result<bool> {
    Ok(get_terracotta(&state)?.verify_room_code(&room_code))
}

#[command]
fn cmd_version(state: State<'_, PluginState>) -> Result<String> {
    Ok(get_terracotta(&state)?.version())
}
