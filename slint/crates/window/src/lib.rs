// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use slint::ComponentHandle;

/// Thin wrapper around a Slint component's window exposing the window-control
/// operations the title bar needs.
///
/// Window-state toggles (minimize / maximize / fullscreen) are wired directly
/// in `.slint` markup via the builtin `Window` properties; this crate exists
/// so Rust code can drive the same operations programmatically (e.g.
/// shortcuts, state restore) without touching the UI tree. It keeps a strong
/// component handle, so it also keeps the event loop alive while live.
pub struct WindowService<H: ComponentHandle> {
    component: Arc<H>,
}

impl<H: ComponentHandle> Clone for WindowService<H> {
    fn clone(&self) -> Self {
        Self {
            component: Arc::clone(&self.component),
        }
    }
}

impl<H: ComponentHandle> WindowService<H> {
    /// Creates a service bound to the given component (strong handle).
    pub fn new(component: H) -> Self {
        Self {
            component: Arc::new(component),
        }
    }

    /// The underlying Slint window.
    pub fn window(&self) -> &slint::Window {
        self.component.window()
    }

    /// Minimizes the window.
    pub fn minimize(&self) {
        self.window().set_minimized(true);
    }

    /// Toggles the maximized state (Windows/Linux).
    pub fn toggle_maximize(&self) {
        self.window().set_maximized(!self.window().is_maximized());
    }

    /// Maximizes (or un-maximizes) the window explicitly.
    pub fn set_maximized(&self, maximized: bool) {
        self.window().set_maximized(maximized);
    }

    /// Toggles fullscreen mode.
    pub fn toggle_fullscreen(&self) {
        self.window().set_fullscreen(!self.window().is_fullscreen());
    }

    /// Reports whether the window is currently maximized.
    pub fn is_maximized(&self) -> bool {
        self.window().is_maximized()
    }

    /// Reports whether the window is currently in fullscreen mode.
    pub fn is_fullscreen(&self) -> bool {
        self.window().is_fullscreen()
    }
}
