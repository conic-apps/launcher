// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Windows window-shell integration.
//!
//! The frameless window (created by the winit hook in [`install_backend`]), the
//! platform's own caption buttons drawn over the app's title bar (see the
//! [`caption`] module), and the Han-script font fallback ([`cjk_font`]).

pub(crate) mod caption;
pub(crate) mod cjk_font;

pub(crate) use cjk_font::apply as apply_cjk_fallbacks;

use slint::ComponentHandle;

use crate::slint_backend::App;

/// Installs the winit window-attributes hook that makes the window frameless.
///
/// Must run before any winit window exists: the client area has to cover the
/// whole window from creation, because Slint only applies `no-frame` once the
/// window exists — and winit would size the first, still-decorated window for a
/// caption that is about to disappear, leaving the app a frame's width and
/// height too big.
pub(crate) fn install_backend() {
    use slint::platform::set_platform;

    let backend = i_slint_backend_winit::Backend::builder()
        .with_window_attributes_hook(|attributes| attributes.with_decorations(false))
        .build()
        .expect("failed to build the winit backend");
    set_platform(Box::new(backend)).expect("failed to install the winit backend");
}

/// Wires the platform's own window controls over the custom title bar.
pub(crate) fn install(ui: &App) {
    // The title bar has to leave room for the controls (see `caption`).
    ui.set_window_controls_inset(caption::controls_inset());

    // The controls take their ink from whatever theme the app resolved to.
    caption::install(ui);
    let weak = ui.as_weak();
    ui.on_set_caption_dark(move |dark| {
        if let Some(ui) = weak.upgrade() {
            caption::set_dark(ui.window(), dark);
        }
    });
}
