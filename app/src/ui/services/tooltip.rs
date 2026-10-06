// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The generic tooltip's pointer position (`ui/globals/tooltip.slint`).
//!
//! Slint has no global cursor accessor. A `TouchArea` knows where the pointer is
//! only while it is the one receiving events, and the tooltip has to follow the
//! pointer no matter which control raised it — including the ones that never see
//! a move. The window does know: winit reports every `CursorMoved`, so this
//! watches the window's events through the shared fan-out
//! (`window::on_window_event`, the same hook the focus watcher uses) and writes
//! both coordinates into `TooltipState`.
//!
//! The pointer is tracked for the whole session rather than only while a panel
//! is up: `TooltipState` has no way to ask winit for the position once, and the
//! first value a panel read would be stale until the next move.
//!
//! The filter runs on the event loop's thread — the one that owns the window —
//! so the write happens right there, the way the focus watcher does it.

use slint::ComponentHandle;

use crate::slint_backend::{App, TooltipState};

/// Installs the cursor tracker on the app window.
pub fn setup(ui: &App) {
    let weak = ui.as_weak();
    window::on_window_event(ui, move |event| {
        let winit::event::WindowEvent::CursorMoved { position, .. } = event else {
            return;
        };
        let Some(ui) = weak.upgrade() else {
            return;
        };
        // winit reports physical pixels; Slint lays out in logical units.
        let logical = position.to_logical::<f32>(ui.window().scale_factor() as f64);
        let tooltip = ui.global::<TooltipState>();
        tooltip.set_pointer_x(logical.x);
        tooltip.set_pointer_y(logical.y);
    });
}
