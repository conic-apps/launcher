// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Native (platform-specific) window-shell integration.
//!
//! Everything the app has to do differently per OS lives behind this module:
//! the winit window-attributes hook (which has to run before any window exists),
//! the macOS traffic lights and Dock icon, the Windows caption buttons, and the
//! Windows Han-script font fallback. `main` wires the same calls on every
//! platform and is otherwise platform-neutral, and the per-OS modules are only
//! compiled where they apply.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::slint_backend::App;

/// Installs the platform's winit window-attributes hook.
///
/// Must run before any window exists — i.e. before `App::new` — so the window is
/// created with the frame the platform wants. A no-op where the platform needs
/// no hook. See each per-OS module for what the hook does and why it cannot
/// wait.
pub(crate) fn install_backend() {
    #[cfg(target_os = "macos")]
    macos::install_backend();
    #[cfg(target_os = "windows")]
    windows::install_backend();
}

/// Registers the platform's terminate handling.
///
/// On macOS `⌘Q`, the menu's Quit and a system logout terminate the process
/// through AppKit, not through a window close. The platform's own Quit is
/// intercepted where AppKit asks (`applicationShouldTerminate:`) and replayed
/// as `request_close`, so it confirms first exactly like `⌘W`; `run_exit_work`
/// stays the fallback for the terminates that really do end the process.
/// Elsewhere there is no such path the app has to cover — the window close is
/// the only exit — so this does nothing.
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

    // Only Windows draws its own caption; nothing else has one to colour.
    #[cfg(not(target_os = "windows"))]
    ui.on_set_caption_dark(|_| {});
}

/// Applies the Han-script font fallback for the bundled `locale`.
///
/// Windows picks a Japanese font for Han without this (see
/// [`windows::cjk_font`]); macOS's CoreText and Linux's fontconfig already
/// answer with a script-appropriate CJK font, so elsewhere this does nothing.
#[cfg(target_os = "windows")]
pub(crate) use windows::apply_cjk_fallbacks;

/// Applies the Han-script font fallback for the bundled `locale`. A no-op off
/// Windows, where the OS fallback is already script-appropriate.
#[cfg(not(target_os = "windows"))]
pub(crate) fn apply_cjk_fallbacks(_locale: &str) {}
