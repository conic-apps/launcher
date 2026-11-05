// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The generic tooltip's pointer position (`ui/globals/tooltip.slint`).
//!
//! Slint has no global cursor accessor, and a `TouchArea` only knows the pointer
//! while it receives events, so winit's `CursorMoved` fan-out is tracked for the
//! whole session; it runs on the event loop's thread and writes `TooltipState`
//! directly.

use slint::ComponentHandle;

use crate::slint_backend::{App, TooltipState};

pub fn setup(ui: &App) {
    let weak = ui.as_weak();
    log::debug!(target: "shell", "tooltip: tracking the pointer for tooltips");
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
