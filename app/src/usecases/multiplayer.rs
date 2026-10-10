// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The multiplayer session commands.
//!
//! The session state machine, the library download and the event port all live
//! in the `multiplayer` crate; the poll thread reports through the crate's
//! `EventSink`. This module is the UI-neutral seam a frontend drives the session
//! through, so opening, creating, joining and leaving a room do not touch Slint.

use multiplayer::NexusService;

/// Whether the Conic Nexus library is present and valid, which decides the
/// screen the dialog opens on.
pub(crate) async fn library_ready() -> bool {
    // The library logs a checksum mismatch itself; this covers the rest, so a
    // dialog stuck on its download screen has a cause in the log.
    match multiplayer::check_library().await {
        Ok(()) => true,
        Err(error) => {
            log::debug!(
                "the Conic Nexus library is not ready, so the dialog will offer to download it: \
                 {error}"
            );
            false
        }
    }
}

pub(crate) async fn create_room(
    service: &NexusService,
    player_name: &str,
) -> Result<(), multiplayer::Error> {
    service.create_room(Some(player_name), None).await
}

pub(crate) async fn join_room(
    service: &NexusService,
    code: &str,
    player_name: &str,
) -> Result<(), multiplayer::Error> {
    service.join_room(code, Some(player_name)).await
}

pub(crate) async fn leave_room(service: &NexusService) -> Result<(), multiplayer::Error> {
    service.leave_room().await
}
