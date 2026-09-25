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

    /// Starts an OS window drag, for a `data-tauri-drag-region`-style region.
    ///
    /// This is `performWindowDragWithEvent:` on macOS — what Chromium, Electron
    /// and tao (Tauri's own windowing layer) do for their drag regions: the drag
    /// is handed to the platform, which tracks it in its own event loop, so it
    /// keeps working outside the window, over other applications, and with the
    /// system's window snapping, and the window is composited by the OS rather
    /// than repainted here.
    ///
    /// It has to be called while a mouse event is being dispatched: the drag is
    /// started from that event (`NSApp.currentEvent`). Slint runs a `TouchArea`'s
    /// pointer handlers inside the event that produced them, so calling this
    /// from one of those is what makes it work.
    ///
    /// The platform keeps the mouse until the drag is over, so the release (and
    /// therefore Slint's `clicked`) never arrives for the press that started it.
    pub fn drag_window(&self) {
        use i_slint_backend_winit::WinitWindowAccessor;
        let _ = self.window().with_winit_window(|window| {
            if let Err(error) = window.drag_window() {
                log::debug!("the platform did not start a window drag: {error}");
            }
        });
    }
}
