// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! macOS window-shell integration.
//!
//! The Chrome-style window (a native titled window with a transparent,
//! title-less titlebar, created by the winit hook in [`install_backend`]), the
//! traffic lights kept in step with the custom title bar ([`traffic_lights`]),
//! the Dock icon ([`dock_icon`]), and AppKit's terminate requests
//! ([`terminate`]).

mod dock_icon;
mod terminate;
mod traffic_lights;

use crate::slint_backend::App;

/// Installs the winit window-attributes hook and the traffic-light metrics.
///
/// Must run before any winit window exists: the frame-view class is chosen
/// during `NSWindow` initialisation, so a window created earlier would keep
/// AppKit's own frame view and never see the metrics.
pub(crate) fn install_backend() {
    use slint::platform::set_platform;
    use winit::platform::macos::WindowAttributesExtMacOS;

    let backend = i_slint_backend_winit::Backend::builder()
        .with_window_attributes_hook(|attributes| {
            attributes
                .with_titlebar_transparent(true)
                .with_title_hidden(true)
                .with_fullsize_content_view(true)
        })
        .build()
        .expect("failed to build the winit backend");
    set_platform(Box::new(backend)).expect("failed to install the winit backend");

    traffic_lights::install();
}

/// Wires the AppKit traffic lights and the Dock icon.
pub(crate) fn install(ui: &App) {
    // The title bar stops leaving room for the traffic lights while they are
    // hidden by fullscreen.
    traffic_lights::watch_fullscreen(ui);

    // The Dock icon draws from NSApplication, not the window.
    dock_icon::install();
}

/// Routes AppKit's terminate requests (`⌘Q`, the menu's Quit, a system logout)
/// through the same close flow as `⌘W`, and runs `run_exit_work` if AppKit does
/// end up terminating.
pub(crate) fn install_terminate_handler(
    request_close: impl Fn() + 'static,
    run_exit_work: impl Fn() + 'static,
) {
    terminate::install(request_close, run_exit_work);
}
