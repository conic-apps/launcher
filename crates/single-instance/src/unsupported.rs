// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Single-instance guard for a platform that has none.
//!
//! `slint-platform` refuses to report a platform outside Windows, Linux and
//! macOS, so this backend is never reached. It keeps the app startable anyway:
//! there is no lock to take, so every launch claims the role and none of them
//! is reported to another.

use std::sync::mpsc::Sender;

use crate::{AlreadyRunning, Launch};

/// Claims the role unconditionally — there is nothing to claim it from.
pub(crate) fn claim(_launches: Sender<Launch>) -> Result<Guard, AlreadyRunning> {
    log::warn!(
        target: "shell",
        "single instance is not supported on this platform; every launch starts a new window"
    );
    Ok(Guard)
}

/// Holds nothing: the platform has no lock to keep, and nothing can be reported
/// to it.
pub(crate) struct Guard;
