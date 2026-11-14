// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Native (platform-specific) window-shell integration: the winit
//! window-attributes hook, the per-OS window chrome, and the font fallbacks the
//! OS gets wrong. `main` wires the same calls everywhere; each per-OS module is
//! only compiled where it applies.

mod fonts;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::slint_backend::App;

/// Installs the platform's winit window-attributes hook.
///
/// Must run before any window exists — i.e. before `App::new` — so the window is
/// created with the frame the platform wants. A no-op where the platform needs
/// no hook.
pub(crate) fn install_backend() {
    #[cfg(target_os = "macos")]
    macos::install_backend();
    #[cfg(target_os = "windows")]
    windows::install_backend();
    // Logged even where this is a no-op, so "no hook is needed" and "the hook was
    // installed" do not look alike in the log.
    #[cfg(target_os = "macos")]
    log::debug!(target: "shell", "backend hook: macOS transparent title bar");
    #[cfg(target_os = "windows")]
    log::debug!(target: "shell", "backend hook: Windows frameless window");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    log::debug!(target: "shell", "backend hook: none is needed on this platform");
}

/// Registers the platform's terminate handling, so an OS-level quit (`⌘Q`, the
/// application menu's Quit, a system logout) takes the window's close path and
/// confirms first. Only macOS has such a path; elsewhere closing the window is
/// the only exit.
pub(crate) fn install_terminate_handler(
    request_close: impl Fn() + 'static,
    run_exit_work: impl Fn() + 'static,
) {
    #[cfg(target_os = "macos")]
    macos::install_terminate_handler(request_close, run_exit_work);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (request_close, run_exit_work);
    }
}

/// Applies the platform-specific window integration that needs the app
/// component. Run once, after `App::new` and before the event loop.
pub(crate) fn install(ui: &App) {
    #[cfg(target_os = "macos")]
    macos::install(ui);
    #[cfg(target_os = "windows")]
    windows::install(ui);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    log::debug!(target: "shell", "window integration: the title bar draws its own controls");

    // Only Windows draws its own caption; nothing else has one to colour.
    #[cfg(not(target_os = "windows"))]
    ui.on_set_caption_dark(|_| {});
}

/// Applies the symbol fallback and, on Windows, the Han one (see
/// `windows::cjk_font`).
pub(crate) fn apply_font_fallbacks(locale: &str) {
    #[cfg(target_os = "windows")]
    windows::apply_cjk_fallbacks(locale);
    #[cfg(not(target_os = "windows"))]
    let _ = locale;

    fonts::apply();
}
