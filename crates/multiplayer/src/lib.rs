// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    path::PathBuf,
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
use log::info;
use tauri::{
    AppHandle, Emitter, Manager, Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};

use crate::{
    error::Result,
    library::check_library_valid,
    metadata::LIBRARY,
    nexus::{NexusSession, PeerInfo, SessionConfig, SessionEvent, SessionState},
};

pub mod error;
pub mod library;
mod metadata;
pub mod nexus;

const EVENT_CHANNEL: &str = "conic-nexus://event";
const CONIC_NEXUS_EVENT_STATE_CHANGED: i32 = 0;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const RECONCILE_EVERY: u32 = 5;

struct PluginState {
    session: Mutex<Option<Arc<NexusSession>>>,
    poll_thread: Mutex<Option<thread::JoinHandle<()>>>,
    shutdown_flag: Arc<AtomicBool>,
    last_state_version: Arc<Mutex<u64>>,
    abort_handle: Arc<Mutex<Option<AbortHandle>>>,
}

impl Default for PluginState {
    fn default() -> Self {
        PluginState {
            session: Mutex::new(None),
            poll_thread: Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            last_state_version: Arc::new(Mutex::new(0)),
            abort_handle: Arc::new(Mutex::new(None)),
        }
    }
}

impl PluginState {
    fn shutdown(&self) {
        info!("Shutting down multiplayer plugin");
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.poll_thread.lock().expect("Internal error").take() {
            info!("Joining poll thread");
            let _ = handle.join();
        }
        if let Some(session) = self.session.lock().expect("Internal error").take() {
            info!("Destroy nexus session");
            session.destroy();
        }
        info!("Plugin stopped");
    }
}

impl Drop for PluginState {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("multiplayer")
        .invoke_handler(tauri::generate_handler![
            cmd_spawn_download_library_task,
            cmd_cancel_download_library_task,
            cmd_check_library_valid,
            cmd_create_room,
            cmd_join_room,
            cmd_leave_room,
            cmd_get_session_state,
            cmd_query_peers,
            cmd_recent_logs,
            cmd_room_code_is_valid,
            cmd_version,
            cmd_configure,
        ])
        .setup(|app, _| {
            app.manage(PluginState::default());
            Ok(())
        })
        .on_event(|app, event| match event {
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                let state = app.state::<PluginState>();
                state.shutdown();
            }
            _ => {}
        })
        .build()
}

/// Lazily loads the Conic Nexus dynamic library, creates the session and
/// starts the background event polling thread. Idempotent and race-safe.
async fn ensure_session<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, PluginState>,
) -> Result<Arc<NexusSession>> {
    if let Some(session) = state.session.lock().expect("Internal error").as_ref() {
        return Ok(session.clone());
    }

    check_library_valid().await?;
    let path = DATA_LOCATION
        .runtime
        .join("conic-nexus")
        .join(LIBRARY.filename);
    let session = NexusSession::load(&path).await?;
    session.configure(&SessionConfig {
        public_nodes: vec![
            "https://terracotta.glavo.site/acebc7d8-1208-47fd-b212-d03ac49e36e0".to_string(),
            "https://api.qomicex.top/api/nodes/7b6cbc18-3edf-43bb-9c78-69e461b7f8ba".to_string(),
            "https://api.qomicex.top/api/nodes/6e349dee-072e-4a47-9e93-a60165146649".to_string(),
        ],
        data_dir: Some(DATA_LOCATION.runtime.join("conic-nexus")),
        motd: None,
    })?;

    let session = Arc::new(session);
    {
        let mut slot = state.session.lock().expect("Internal error");
        if let Some(existing) = slot.as_ref() {
            session.destroy();
            return Ok(existing.clone());
        }
        *slot = Some(session.clone());
    }

    spawn_poll_thread(app.clone(), session.clone(), state);
    Ok(session)
}

fn spawn_poll_thread<R: Runtime>(
    app: AppHandle<R>,
    session: Arc<NexusSession>,
    state: &State<'_, PluginState>,
) {
    let mut slot = state.poll_thread.lock().expect("Internal error");
    if slot.is_some() {
        return;
    }
    *slot = Some(thread::spawn({
        let shutdown = state.shutdown_flag.clone();
        let last_state_version = state.last_state_version.clone();
        move || poll_loop(app, session, shutdown, last_state_version)
    }));
}

/// Drains the Conic Nexus event queue and re-emits every notice as a Tauri
/// event. A periodic `get_state` reconciliation covers events dropped by the
/// bounded queue by synthesising a STATE_CHANGED notice when the version
/// advanced without us seeing it.
fn poll_loop<R: Runtime>(
    app: AppHandle<R>,
    session: Arc<NexusSession>,
    shutdown: Arc<AtomicBool>,
    last_state_version: Arc<Mutex<u64>>,
) {
    let mut reconcile_ticks: u32 = 0;
    while !shutdown.load(Ordering::SeqCst) {
        loop {
            match session.poll_event() {
                Ok(Some(event)) => {
                    if event.r#type == CONIC_NEXUS_EVENT_STATE_CHANGED
                        && let Some(version) = event.payload.get("version").and_then(|v| v.as_u64())
                    {
                        let mut last = last_state_version.lock().expect("Internal error");
                        if version > *last {
                            *last = version;
                        }
                    }
                    let _ = app.emit(EVENT_CHANNEL, &event);
                }
                Ok(None) => break,
                Err(error) => {
                    log::error!("Conic Nexus poll_event failed: {error}");
                    break;
                }
            }
        }

        reconcile_ticks += 1;
        if reconcile_ticks >= RECONCILE_EVERY {
            reconcile_ticks = 0;
            if let Ok(state) = session.get_state() {
                let mut last = last_state_version.lock().expect("Internal error");
                if state.version > *last {
                    *last = state.version;
                    let event = SessionEvent {
                        sequence: 0,
                        r#type: CONIC_NEXUS_EVENT_STATE_CHANGED,
                        payload: serde_json::json!({
                            "state": state.state,
                            "version": state.version,
                        }),
                    };
                    let _ = app.emit(EVENT_CHANNEL, &event);
                }
            }
        }

        thread::sleep(EVENT_POLL_INTERVAL);
    }
}

fn library_dir() -> PathBuf {
    DATA_LOCATION.runtime.join("conic-nexus")
}

#[command]
async fn cmd_spawn_download_library_task(
    state: State<'_, PluginState>,
    channel: Channel<DownloadState>,
) -> Result<()> {
    let progress = DownloadState::default();
    let (handle, reg) = AbortHandle::new_pair();
    let future = Abortable::new(crate::library::download_library(&progress), reg);
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

#[command]
async fn cmd_create_room<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    player_name: Option<String>,
    room_code: Option<String>,
) -> Result<()> {
    let session = ensure_session(&app, &state).await?;
    session.create_room(player_name.as_deref(), room_code.as_deref())
}

#[command]
async fn cmd_join_room<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    room_code: String,
    player_name: Option<String>,
) -> Result<()> {
    let session = ensure_session(&app, &state).await?;
    session.join_room(&room_code, player_name.as_deref())
}

#[command]
async fn cmd_leave_room<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
) -> Result<()> {
    let session = ensure_session(&app, &state).await?;
    session.reset_to_waiting()
}

#[command]
async fn cmd_get_session_state<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
) -> Result<SessionState> {
    let session = ensure_session(&app, &state).await?;
    session.get_state()
}

#[command]
async fn cmd_query_peers<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
) -> Result<Vec<PeerInfo>> {
    let session = ensure_session(&app, &state).await?;
    session.query_peers()
}

#[command]
async fn cmd_recent_logs<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    limit: Option<u32>,
) -> Result<Vec<String>> {
    let session = ensure_session(&app, &state).await?;
    session.recent_logs(limit.unwrap_or(100))
}

#[command]
async fn cmd_room_code_is_valid<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    room_code: String,
) -> Result<bool> {
    let session = ensure_session(&app, &state).await?;
    Ok(session.room_code_is_valid(&room_code))
}

#[command]
async fn cmd_version<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
) -> Result<String> {
    let session = ensure_session(&app, &state).await?;
    Ok(session.version())
}

#[command]
async fn cmd_configure<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    public_nodes: Option<Vec<String>>,
    data_dir: Option<String>,
    motd: Option<String>,
) -> Result<()> {
    let session = ensure_session(&app, &state).await?;
    session.configure(&SessionConfig {
        public_nodes: public_nodes.unwrap_or_default(),
        data_dir: data_dir.map(PathBuf::from).or_else(|| Some(library_dir())),
        motd,
    })
}
