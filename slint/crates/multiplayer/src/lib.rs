// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/multiplayer`: the Conic Nexus cross-LAN
//! multiplayer session.
//!
//! The original exposes its work through Tauri commands and keeps the live
//! session, its background poll thread and the download abort handle in a
//! `PluginState`; every notice the Conic Nexus library produces is re-emitted as
//! a Tauri event on `conic-nexus://event`. This mirror keeps the whole domain
//! layer (`nexus.rs`, `library.rs`, `metadata.rs` and `error.rs` are the
//! originals save for the `shared` / `folder` / `download` to `slint-*`
//! renames) and folds the command layer into [`NexusService`]:
//!
//!   * [`NexusService`] is the `PluginState`: it lazily loads the library,
//!     creates and configures the session and runs the poll loop with the same
//!     reconciliation;
//!   * the notices that used to be `app.emit(EVENT_CHANNEL, ...)` are handed to
//!     an [`EventSink`] the app installs, which plays the role of the Tauri
//!     event bus (the Slint app forwards them into the UI on the event loop);
//!   * the download task's `AbortHandle` that lived in the state is owned by the
//!     app, which aborts the tokio task it spawned (exactly like the launch
//!     view's cancel handles).

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use log::info;

use crate::{error::Result, library::check_library_valid, metadata::LIBRARY};

pub mod error;
pub mod library;
mod metadata;
pub mod nexus;
pub mod room_code;

pub use error::Error;
pub use library::{check_library_valid as check_library, download_library, library_dir};
pub use nexus::{NexusSession, PeerInfo, SessionConfig, SessionEvent, SessionState};
pub use room_code::is_room_code_valid;

/// The Tauri event name the original re-emits notices on, kept for parity: the
/// Slint app routes the same notices to the UI through its [`EventSink`].
pub const EVENT_CHANNEL: &str = "conic-nexus://event";
/// `CONIC_NEXUS_EVENT_STATE_CHANGED` is the one event type the poll loop reads a
/// version out of when it reconciles.
pub const CONIC_NEXUS_EVENT_STATE_CHANGED: i32 = 0;
pub const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
pub const RECONCILE_EVERY: u32 = 5;

/// Receives every notice the poll loop drains, standing in for the Tauri event
/// bus. It is called from the poll thread, so whatever it closes over has to be
/// `Send + Sync`; the app marshals the event onto the Slint event loop itself.
pub type EventSink = Arc<dyn Fn(SessionEvent) + Send + Sync + 'static>;

/// The original's `PluginState`: the lazily loaded session, its background poll
/// thread and the reconciler's bookkeeping.
pub struct NexusService {
    session: Mutex<Option<Arc<NexusSession>>>,
    poll_thread: Mutex<Option<thread::JoinHandle<()>>>,
    shutdown_flag: Arc<AtomicBool>,
    last_state_version: Arc<Mutex<u64>>,
    events: EventSink,
}

impl NexusService {
    /// Builds the service. `events` is called from the poll thread for every
    /// drained notice.
    pub fn new(events: EventSink) -> Self {
        NexusService {
            session: Mutex::new(None),
            poll_thread: Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            last_state_version: Arc::new(Mutex::new(0)),
            events,
        }
    }

    /// Stops the poll thread and destroys the session (the original's
    /// `PluginState::shutdown`, run there on the app's exit event).
    pub fn shutdown(&self) {
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

    /// The session, loading the library and starting the poll loop on first
    /// use. Idempotent and race-safe, like the original's `ensure_session`.
    pub async fn session(&self) -> Result<Arc<NexusSession>> {
        if let Some(session) = self.session.lock().expect("Internal error").as_ref() {
            return Ok(session.clone());
        }

        check_library_valid().await?;
        let path = library_dir().join(LIBRARY.filename);
        let session = NexusSession::load(&path).await?;
        session.configure(&SessionConfig {
            public_nodes: vec![
                // NOTE: add other nodes here
            ],
            data_dir: Some(library_dir()),
            motd: None,
        })?;

        let session = Arc::new(session);
        {
            let mut slot = self.session.lock().expect("Internal error");
            if let Some(existing) = slot.as_ref() {
                session.destroy();
                return Ok(existing.clone());
            }
            *slot = Some(session.clone());
        }

        self.spawn_poll_thread(session.clone());
        Ok(session)
    }

    /// Starts the session if it is not running yet.
    pub async fn ensure_started(&self) -> Result<()> {
        self.session().await.map(|_| ())
    }

    /// The `cmd_create_room` command.
    pub async fn create_room(
        &self,
        player_name: Option<&str>,
        room_code: Option<&str>,
    ) -> Result<()> {
        let session = self.session().await?;
        session.create_room(player_name, room_code)
    }

    /// The `cmd_join_room` command.
    pub async fn join_room(&self, room_code: &str, player_name: Option<&str>) -> Result<()> {
        let session = self.session().await?;
        session.join_room(room_code, player_name)
    }

    /// The `cmd_leave_room` command.
    pub async fn leave_room(&self) -> Result<()> {
        let session = self.session().await?;
        session.reset_to_waiting()
    }

    /// The `cmd_get_session_state` command.
    pub async fn get_session_state(&self) -> Result<SessionState> {
        let session = self.session().await?;
        session.get_state()
    }

    /// The `cmd_query_peers` command.
    pub async fn query_peers(&self) -> Result<Vec<PeerInfo>> {
        let session = self.session().await?;
        session.query_peers()
    }

    /// The `cmd_recent_logs` command.
    pub async fn recent_logs(&self, limit: Option<u32>) -> Result<Vec<String>> {
        let session = self.session().await?;
        session.recent_logs(limit.unwrap_or(100))
    }

    /// The `cmd_room_code_is_valid` command: the library's own check, as
    /// opposed to the standalone [`is_room_code_valid`] the UI uses.
    pub async fn room_code_is_valid(&self, room_code: &str) -> Result<bool> {
        let session = self.session().await?;
        Ok(session.room_code_is_valid(room_code))
    }

    /// The `cmd_version` command.
    pub async fn version(&self) -> Result<String> {
        let session = self.session().await?;
        Ok(session.version())
    }

    /// The `cmd_configure` command.
    pub async fn configure(&self, config: &SessionConfig) -> Result<()> {
        let session = self.session().await?;
        session.configure(config)
    }

    /// Starts the event poll thread (the original's `spawn_poll_thread`).
    fn spawn_poll_thread(&self, session: Arc<NexusSession>) {
        let mut slot = self.poll_thread.lock().expect("Internal error");
        if slot.is_some() {
            return;
        }
        *slot = Some(thread::spawn({
            let shutdown = self.shutdown_flag.clone();
            let last_state_version = self.last_state_version.clone();
            let events = self.events.clone();
            move || poll_loop(session, shutdown, last_state_version, events)
        }));
    }
}

impl Drop for NexusService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Drains the Conic Nexus event queue and hands every notice to the sink. A
/// periodic `get_state` reconciliation covers events dropped by the bounded
/// queue by synthesising a STATE_CHANGED notice when the version advanced
/// without us seeing it (the original's `poll_loop`, with `app.emit` replaced
/// by the sink).
fn poll_loop(
    session: Arc<NexusSession>,
    shutdown: Arc<AtomicBool>,
    last_state_version: Arc<Mutex<u64>>,
    events: EventSink,
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
                    events(event);
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
                    events(event);
                }
            }
        }

        thread::sleep(EVENT_POLL_INTERVAL);
    }
}
