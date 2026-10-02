// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use slint::ComponentHandle;

/// One thing watching the app window's winit events. See [`on_window_event`].
type EventFilter = Box<dyn FnMut(&winit::event::WindowEvent)>;

thread_local! {
    /// Everyone watching, in the order they asked.
    static EVENT_FILTERS: RefCell<Vec<EventFilter>> = const { RefCell::new(Vec::new()) };
    /// Whether the one filter the winit backend accepts has been registered.
    static EVENT_HOOK: Cell<bool> = const { Cell::new(false) };
}

/// Registers `filter` to see every winit window event the app window gets.
///
/// **The winit backend keeps exactly one event filter per Slint window, and
/// `on_winit_window_event` *replaces* it** rather than adding to it — the slot
/// is a `Cell<Option<Box<dyn FnMut(..)>>>`. So the second caller of the raw API
/// silently unhooks the first, and the symptom is every watcher but the one
/// that registered last simply never running: that is how the window controls
/// lost their hook (`windows_caption`) and the background lost its pointer.
///
/// This fans every subscriber out through the one slot, so the order they
/// register in does not decide which of them works. Call it instead.
///
/// Events are always propagated: nothing in the app consumes one, and a filter
/// that stopped propagation would take the event away from all the others and
/// from Slint's own handling as well.
///
/// The filters run on the thread that owns the window — the one running the
/// event loop — which is why the list is thread-local and needs no locking.
/// The window is looked up once, on the first registration, because a winit
/// window does not exist until the loop has created it; there is only one, and
/// the app has one window.
pub fn on_window_event<H: ComponentHandle>(
    component: &H,
    filter: impl FnMut(&winit::event::WindowEvent) + 'static,
) {
    use i_slint_backend_winit::{EventResult, WinitWindowAccessor};

    EVENT_FILTERS.with(|filters| filters.borrow_mut().push(Box::new(filter)));

    if EVENT_HOOK.replace(true) {
        // The slot already holds the dispatcher, and it reads the list above,
        // so a later subscriber is picked up without touching it again.
        return;
    }
    component.window().on_winit_window_event(|_, event| {
        EVENT_FILTERS.with(|filters| {
            // The borrow is held across the calls on purpose: a filter cannot be
            // removed while it is running, and none of them can reach back in
            // here (the backend takes its own slot out for the duration of the
            // dispatch, so a registration from inside would be lost anyway).
            for filter in filters.borrow_mut().iter_mut() {
                filter(event);
            }
        });
        EventResult::Propagate
    });
}

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

    /// A weak handle to the component, for a callback that has to reach the UI
    /// from somewhere the service cannot be moved (a focus listener, a watcher).
    pub fn component_weak(&self) -> slint::Weak<H>
    where
        H: slint::ComponentHandle,
    {
        self.component.as_weak()
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

    /// Reports every focus change to `callback`.
    ///
    /// winit's `WindowEvent::Focused` is the platform's own answer, and it is
    /// delivered to a filter installed here rather than polled: a poll would have
    /// to run often enough not to miss a short trip to another window, and the
    /// volume ramp the music player follows a focus change with is not something
    /// to discover half a second late.
    ///
    /// There is deliberately no matching "read it once" accessor. A read at
    /// startup would always answer "focused", because a Slint window does not
    /// exist until the event loop runs, so the first event is what actually
    /// settles it.
    ///
    /// The callback runs on the event loop's thread, and the event is propagated
    /// afterwards so Slint still sees it.
    pub fn on_focus_changed(&self, callback: impl FnMut(bool) + 'static) {
        let mut callback = callback;
        on_window_event(self.component.as_ref(), move |event| {
            if let winit::event::WindowEvent::Focused(focused) = event {
                callback(*focused);
            }
        });
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
    /// running (`single-instance`): the user asked for the launcher again, and
    /// the answer is the window they already have, not a second one.
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

    /// Starts an OS window drag, for a title-bar drag region.
    ///
    /// This is `performWindowDragWithEvent:` on macOS — what Chromium and
    /// Electron do for their drag regions: the drag is handed to the platform,
    /// which tracks it in its own event loop, so it keeps working outside the
    /// window, over other applications, and with the system's window snapping,
    /// and the window is composited by the OS rather than repainted here.
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
