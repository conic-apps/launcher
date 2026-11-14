// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance settings overlay: it owns the instance being edited, turns the
//! UI's edits into an `InstanceConfig` and writes `instance.toml` on every edit.
//! The delete dialog is wired here too, since it uses the same instance.

use std::{cell::RefCell, rc::Rc, time::Duration};

use slint::{ComponentHandle, Image, SharedPixelBuffer, Timer, TimerMode};

use crate::slint_backend::{App, DeleteInstanceState, Dialogs, GameState, InstanceSettingsState};
use crate::ui::views::game::format_play_time;
use config::launch::{LaunchConfig, Server};
use instance::{Instance, InstanceConfig, InstanceLaunchConfig};

mod dialogs;
mod form;

pub(crate) use dialogs::*;
pub(crate) use form::*;

/// How long an edit waits before it reaches `instance.toml`: what is left after
/// the UI thread's synchronous write is the cost of one `toml` write per keystroke.
pub(crate) const SAVE_DEBOUNCE: Duration = Duration::from_millis(400);
