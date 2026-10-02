// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Conic Nexus cross-LAN multiplayer session.
//!
//! [`NexusService`] owns the domain layer: it lazily loads the library, creates
//! and configures the session, and runs the background poll loop that drains the
//! library's event queue and reconciles its state. Every notice is handed to an
//! [`EventSink`] the app installs, and the app forwards them into the UI on the
//! Slint event loop. The download task's abort handle is owned by the app, which
//! aborts the tokio task it spawned (like the launch view's cancel handles).

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
pub use library::{
    LibrarySink, check_library_valid as check_library, download_library, library_dir,
};
pub use nexus::{NexusSession, PeerInfo, SessionConfig, SessionEvent, SessionState};
pub use room_code::is_room_code_valid;

/// The channel name the app routes Conic Nexus notices under, through its
/// [`EventSink`].
pub const EVENT_CHANNEL: &str = "conic-nexus://event";
/// `CONIC_NEXUS_EVENT_STATE_CHANGED` is the one event type the poll loop reads a
/// version out of when it reconciles.
pub const CONIC_NEXUS_EVENT_STATE_CHANGED: i32 = 0;
pub const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
pub const RECONCILE_EVERY: u32 = 5;

/// Receives every notice the poll loop drains. It is called from the poll
/// thread, so whatever it closes over has to be `Send + Sync`; the app marshals
/// the event onto the Slint event loop itself.
pub type EventSink = shared::Sink<SessionEvent>;

/// The lazily loaded session, its background poll thread and the reconciler's
/// bookkeeping.
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

    /// Stops the poll thread and destroys the session. Called on the app's exit
    /// event.
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
    /// use. Idempotent and race-safe.
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

    /// Creates a room, minting a code when none is given.
    pub async fn create_room(
        &self,
        player_name: Option<&str>,
        room_code: Option<&str>,
    ) -> Result<()> {
        let session = self.session().await?;
        session.create_room(player_name, room_code)
    }

    /// Joins an existing room.
    pub async fn join_room(&self, room_code: &str, player_name: Option<&str>) -> Result<()> {
        let session = self.session().await?;
        session.join_room(room_code, player_name)
    }

    /// Leaves the room and returns the session to waiting.
    pub async fn leave_room(&self) -> Result<()> {
        let session = self.session().await?;
        session.reset_to_waiting()
    }

    /// The current session snapshot.
    pub async fn get_session_state(&self) -> Result<SessionState> {
        let session = self.session().await?;
        session.get_state()
    }

    /// The active mesh's peers.
    pub async fn query_peers(&self) -> Result<Vec<PeerInfo>> {
        let session = self.session().await?;
        session.query_peers()
    }

    /// The most recent log lines.
    pub async fn recent_logs(&self, limit: Option<u32>) -> Result<Vec<String>> {
        let session = self.session().await?;
        session.recent_logs(limit.unwrap_or(100))
    }

    /// Whether `room_code` is well-formed, using the library's own check, as
    /// opposed to the standalone [`is_room_code_valid`] the UI uses.
    pub async fn room_code_is_valid(&self, room_code: &str) -> Result<bool> {
        let session = self.session().await?;
        Ok(session.room_code_is_valid(room_code))
    }

    /// The library's static version string.
    pub async fn version(&self) -> Result<String> {
        let session = self.session().await?;
        Ok(session.version())
    }

    /// Applies a startup configuration.
    pub async fn configure(&self, config: &SessionConfig) -> Result<()> {
        let session = self.session().await?;
        session.configure(config)
    }

    /// Starts the event poll thread, unless one is already running.
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
/// without the queue reporting it.
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
