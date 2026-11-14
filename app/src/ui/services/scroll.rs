// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Provides the smooth-scroll components (`ui/globals/scroll.slint`) with a
//! monotonic clock and a wheel/trackpad/momentum classification that Slint's
//! `PointerScrollEvent` cannot carry.

use std::cell::Cell;
use std::sync::OnceLock;
use std::time::Instant;

use slint::ComponentHandle;

use crate::slint_backend::{App, ScrollInput, ScrollSource};

/// The observer runs only microseconds before the handler reads it, so a
/// classification older than this belongs to an earlier event; a stale
/// `trackpad` would make a wheel notch jump instead of glide.
const FRESH_MS: u64 = 250;

thread_local! {
    /// The last classification recorded by the platform observer, and when. Both
    /// ends run on the main thread: the observer is called from event dispatch and
    /// the handlers read it inside Slint's dispatch of that same event.
    static LAST: Cell<Option<(ScrollSource, Instant)>> = const { Cell::new(None) };
}

pub fn setup(ui: &App) {
    let input = ui.global::<ScrollInput>();

    // Monotonic on purpose: a wall clock can be stepped (NTP), which would make
    // a frame's step jump or stall.
    input.on_now_ms(|| {
        static ORIGIN: OnceLock<Instant> = OnceLock::new();
        ORIGIN.get_or_init(Instant::now).elapsed().as_secs_f32() * 1000.0
    });

    input.on_source(|| {
        LAST.with(|last| match last.get() {
            Some((source, at)) if at.elapsed().as_millis() as u64 <= FRESH_MS => source,
            _ => ScrollSource::Wheel,
        })
    });

    #[cfg(target_os = "macos")]
    macos::install();
}

/// Records the observer's last classification.
///
/// macOS-only: the submodule is the only caller, and on other platforms the
/// dead code would fail CI's `clippy -- -D warnings`.
#[cfg(target_os = "macos")]
fn record(source: ScrollSource) {
    LAST.with(|last| {
        if last.get().map(|(previous, _)| previous) != Some(source) {
            log::debug!(target: "shell", "scroll: input is now {source:?}");
        }
        last.set(Some((source, Instant::now())));
    });
}

/// macOS scroll classifier: an `NSEvent` local monitor, the supported hook for
/// seeing events before they reach the window (unlike a swizzle, it needs no
/// permission and replaces nothing). `hasPreciseScrollingDeltas` marks trackpad
/// and Magic Mouse, `momentumPhase` the synthesized post-finger events.
#[cfg(target_os = "macos")]
mod macos {
    use block2::RcBlock;
    use objc2::class;
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    use crate::slint_backend::ScrollSource;

    /// `NSEventMaskScrollWheel` — `1 << NSEventTypeScrollWheel`, which is 22.
    /// Spelled out because the mask type lives in AppKit, which this crate does
    /// not depend on: the classes below are reached through the runtime.
    const SCROLL_WHEEL_MASK: usize = 1 << 22;

    pub fn install() {
        // The handler only records; the event is returned unchanged so AppKit
        // dispatches it as if nothing had looked at it.
        let handler = RcBlock::new(move |event: *mut AnyObject| -> *mut AnyObject {
            if !event.is_null() {
                // SAFETY: the observer is only registered for scroll-wheel
                // events, so `event` is an `NSEvent` of that type — the reads
                // `classify` makes are AppKit's own accessors on it.
                super::record(unsafe { classify(event) });
            }
            event
        });

        // SAFETY: called on the main thread during startup, with a block whose
        // signature matches the one AppKit calls. The returned monitor object is
        // deliberately dropped: the observer is meant to live as long as the
        // process does, and it is the notification centre that keeps the copied
        // block alive. `[NSEvent momentumPhase]` and `[NSEvent
        // hasPreciseScrollingDeltas]` are long-standing AppKit API on an
        // `NSEvent` the handler was handed.
        unsafe {
            let _monitor: *mut AnyObject = msg_send![
                class!(NSEvent),
                addLocalMonitorForEventsMatchingMask: SCROLL_WHEEL_MASK,
                handler: &*handler
            ];
        }
        log::debug!(target: "shell", "scroll: watching AppKit scroll events");
    }

    unsafe fn classify(event: *mut AnyObject) -> ScrollSource {
        // SAFETY: the observer is only registered for scroll-wheel events, so
        // `event` is an `NSEvent` of that type.
        unsafe {
            let momentum: usize = msg_send![event, momentumPhase];
            if momentum != 0 {
                return ScrollSource::Momentum;
            }
            let precise: bool = msg_send![event, hasPreciseScrollingDeltas];
            if precise {
                ScrollSource::Trackpad
            } else {
                ScrollSource::Wheel
            }
        }
    }
}
