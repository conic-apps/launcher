// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance settings overlay's "script": it owns the instance the overlay
//! is editing, turns the UI's edits into an `InstanceConfig` and writes
//! `instance.toml` on every edit.
//!
//! The delete dialog the overlay opens is wired here too, since it is handed
//! the same instance.

use std::{cell::RefCell, rc::Rc, time::Duration};

use slint::{ComponentHandle, Image, SharedPixelBuffer, Timer, TimerMode};

use crate::config_bridge;
use crate::game::format_play_time;
use crate::slint_backend::{App, DeleteInstanceState, Dialogs, GameState, InstanceSettingsState};
use config::launch::{LaunchConfig, Server};
use instance::{Instance, InstanceConfig, InstanceLaunchConfig};

mod dialogs;
mod form;

pub(crate) use dialogs::*;
pub(crate) use form::*;

/// How long an edit waits before it reaches `instance.toml`.
///
/// A Slint write is a synchronous call on the UI thread, so there is no window
/// for a second edit to slip into and nothing to lock; what is left is the cost
/// of one `toml` write per keystroke, which the 400 ms the settings screen
/// already uses absorbs.
pub(crate) const SAVE_DEBOUNCE: Duration = Duration::from_millis(400);
