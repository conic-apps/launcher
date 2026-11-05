// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use slint::ComponentHandle;

type EventFilter = Box<dyn FnMut(&winit::event::WindowEvent)>;

thread_local! {
    static EVENT_FILTERS: RefCell<Vec<EventFilter>> = const { RefCell::new(Vec::new()) };
    static EVENT_HOOK: Cell<bool> = const { Cell::new(false) };
}

/// Registers `filter` to see every winit window event the app window gets.
///
/// The winit backend has one filter slot per window and `on_winit_window_event`
/// replaces it, so this fans subscribers out through that slot. Events are
/// always propagated so the other filters and Slint still see them. The list is
/// thread-local because filters run on the event loop's thread.
pub fn on_window_event<H: ComponentHandle>(
    component: &H,
    filter: impl FnMut(&winit::event::WindowEvent) + 'static,
) {
    use i_slint_backend_winit::{EventResult, WinitWindowAccessor};

    EVENT_FILTERS.with(|filters| {
        let mut filters = filters.borrow_mut();
        log::debug!(
            "attaching a window event filter; {} will now be dispatched",
            filters.len() + 1
        );
        filters.push(Box::new(filter));
    });

    if EVENT_HOOK.replace(true) {
        return;
    }
    component.window().on_winit_window_event(|_, event| {
        EVENT_FILTERS.with(|filters| {
            // Held across the calls: a filter cannot be removed while running,
            // and the backend already took its slot out for this dispatch.
            for filter in filters.borrow_mut().iter_mut() {
                filter(event);
            }
        });
        EventResult::Propagate
    });
}

/// Window-control operations the title bar needs. Holds a strong component
/// handle, so it keeps the event loop alive while live.
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
    pub fn new(component: H) -> Self {
        Self {
            component: Arc::new(component),
        }
    }

    pub fn window(&self) -> &slint::Window {
        self.component.window()
    }

    pub fn component_weak(&self) -> slint::Weak<H>
    where
        H: slint::ComponentHandle,
    {
        self.component.as_weak()
    }

    pub fn minimize(&self) {
        self.set_minimized(true);
    }

    pub fn toggle_maximize(&self) {
        let maximized = !self.window().is_maximized();
        self.set_maximized(maximized);
    }

    /// Slint's `set_maximized` reports no outcome, so this logs the request
    /// itself; otherwise a refused maximize looks like a lost click.
    pub fn set_maximized(&self, maximized: bool) {
        self.window().set_maximized(maximized);
        log::debug!("the window was asked to be maximized={maximized}");
    }

    pub fn toggle_fullscreen(&self) {
        let fullscreen = !self.window().is_fullscreen();
        self.window().set_fullscreen(fullscreen);
        log::debug!("the window was asked to be fullscreen={fullscreen}");
    }

    /// Reports every focus change to `callback` on the event loop's thread.
    ///
    /// An event rather than a poll, so a short trip to another window is not
    /// missed. There is no "read it once" accessor: before the event loop runs
    /// there is no Slint window, so it would always answer "focused".
    pub fn on_focus_changed(&self, callback: impl FnMut(bool) + 'static) {
        let mut callback = callback;
        on_window_event(self.component.as_ref(), move |event| {
            if let winit::event::WindowEvent::Focused(focused) = event {
                callback(*focused);
            }
        });
    }

    pub fn is_maximized(&self) -> bool {
        self.window().is_maximized()
    }

    pub fn is_fullscreen(&self) -> bool {
        self.window().is_fullscreen()
    }

    /// Brings the window forward and gives it input focus. This is what a
    /// second launch does to the instance already running.
    ///
    /// A minimized or hidden window is restored first: the platform cannot
    /// activate a window that is not on screen. Must run on the event loop's
    /// thread.
    pub fn bring_to_front(&self) {
        log::debug!("bringing the window forward");
        if self.window().is_minimized() {
            self.set_minimized(false);
        }
        if !self.window().is_visible()
            && let Err(error) = self.window().show()
        {
            log::debug!("the window could not be shown: {error}");
        }
        self.focus_window();
    }

    fn set_minimized(&self, minimized: bool) {
        use i_slint_backend_winit::WinitWindowAccessor;
        match self
            .window()
            .with_winit_window(|window| window.set_minimized(minimized))
        {
            Some(_) => log::debug!("the window manager was asked for minimized={minimized}"),
            None => {
                log::debug!(
                    "no winit window to minimize; falling back to Slint for minimized={minimized}"
                );
                self.window().set_minimized(minimized);
            }
        }
    }

    fn focus_window(&self) {
        use i_slint_backend_winit::WinitWindowAccessor;
        let focused = self
            .window()
            .with_winit_window(|window| window.focus_window());
        if focused.is_none() {
            log::debug!("the window could not be focused: no winit window to focus");
        }
    }

    /// Starts an OS window drag from a title-bar drag region. The platform
    /// tracks the drag, so it keeps working outside the window.
    ///
    /// Must be called while a mouse event is being dispatched: macOS starts the
    /// drag from the current event. The platform keeps the mouse until release,
    /// so Slint never sees the `clicked` for the press that started it.
    pub fn drag_window(&self) {
        use i_slint_backend_winit::WinitWindowAccessor;
        if self
            .window()
            .with_winit_window(|window| {
                if let Err(error) = window.drag_window() {
                    log::debug!("the platform did not start a window drag: {error}");
                }
            })
            .is_none()
        {
            log::debug!("the window drag did not start: no winit window");
        }
    }
}
