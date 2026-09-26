// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use slint::ComponentHandle;

/// Thin wrapper around a Slint component's window exposing the window-control
/// operations the title bar needs.
///
/// The title bar and Rust callbacks use this service to drive window-state
/// operations without reaching into the UI tree. It keeps a strong component
/// handle, so it also keeps the event loop alive while live.
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
        self.minimize_to(true);
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

    /// Brings the window back in front of the user and gives it the input focus.
    ///
    /// This is what a second launch of the app does to the one that is already
    /// running (`slint-single-instance`): the user asked for the launcher
    /// again, and the answer is the window they already have, not a second one.
    ///
    /// A minimized window is restored first, because the platform's own
    /// activation does nothing for one — it is not on screen to bring forward.
    /// That is the case when the user reaches the launcher from the dock or the
    /// taskbar, and minimized is where it usually is by then.
    ///
    /// The underlying calls take the focus from whatever the user is working
    /// in, which is the intent here and why nothing in the app calls this
    /// speculatively.
    ///
    /// It has to run on the event loop's thread: the winit window only exists
    /// while the loop is pumping, and the platform's activation has to run where
    /// its own window lives. Call it from an `upgrade_in_event_loop` callback.
    pub fn bring_to_front(&self) {
        if self.window().is_minimized() {
            self.minimize_to(false);
        }
        if !self.window().is_visible() {
            // A launch of the app that arrives after the window was hidden — a
            // window that is not on screen cannot be brought to it.
            if let Err(error) = self.window().show() {
                log::debug!("the window could not be shown: {error}");
            }
        }
        self.focus();
    }

    /// Restores or minimizes the window.
    fn minimize_to(&self, minimized: bool) {
        use i_slint_backend_winit::WinitWindowAccessor;
        if self
            .window()
            .with_winit_window(|window| window.set_minimized(minimized))
            .is_none()
        {
            self.window().set_minimized(minimized);
        }
    }

    /// Asks the platform to make the window the focused one.
    fn focus(&self) {
        use i_slint_backend_winit::WinitWindowAccessor;
        let focused = self
            .window()
            .with_winit_window(|window| window.focus_window());
        if focused.is_none() {
            // No winit window (another backend): Slint has no activation of its
            // own, so there is nothing left to try.
            log::debug!("the window could not be focused: no winit window to focus");
        }
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
