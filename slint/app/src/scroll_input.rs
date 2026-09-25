// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The two platform facts the smooth-scroll components are built on
//! (`ui/globals/scroll.slint`): a monotonic clock, and the classification of a
//! scroll event as wheel / trackpad / trackpad momentum.
//!
//! # Why the classification is not in `.slint`
//!
//! Slint hands a scroll event over as a `PointerScrollEvent`, which carries the
//! deltas and the modifiers and nothing else. It therefore cannot say whether
//! the event came from a wheel — a distance to ease the offset towards — or from
//! a trackpad, whose deltas are a motion the gesture is still making and have to
//! be followed (the fingers, and the momentum macOS derives from the velocity
//! they were moving at when they left; see `ui/globals/scroll.slint`). AppKit's
//! two answers for that, `hasPreciseScrollingDeltas` and `momentumPhase`, are
//! both dropped on the way in: winit keeps the `NSEventPhase` but folds
//! `momentumPhase` into it, and a momentum gesture starts immediately after the
//! finger gesture ends, so the phase alone cannot tell the two apart.
//!
//! # How it is read
//!
//! The components *pull*: a `scroll-event` handler asks for the source while it
//! handles the event. On macOS an `NSEvent` observer records the classification
//! as the event goes by, before Slint sees it (see [`macos`]), so the answer
//! belongs to the very event being handled. Everything else — every other
//! platform, and any reading that is not fresh — is `wheel`, which is the
//! Lenis-smoothed behaviour those platforms had before this existed.

use std::cell::Cell;
use std::sync::OnceLock;
use std::time::Instant;

use slint::ComponentHandle;

use crate::slint_backend::{App, ScrollInput, ScrollSource};

/// How long a recorded classification stays valid, in milliseconds.
///
/// The observer runs a few microseconds before the handler that reads it, so
/// this only has to outlast the hand-off to Slint's input dispatch. Anything
/// older is left over from an earlier event and must not be trusted — a stale
/// `trackpad` would make a wheel notch jump instead of glide.
const FRESH_MS: u64 = 250;

thread_local! {
    /// The last classification recorded by the platform observer, and when. Both
    /// ends run on the main thread: the observer is called from the event
    /// dispatch, and the handlers that read it are running inside Slint's
    /// dispatch of that same event.
    static LAST: Cell<Option<(ScrollSource, Instant)>> = const { Cell::new(None) };
}

/// Wires the `ScrollInput` global and installs the platform observer, if the
/// platform has one.
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

/// Records what the observer last saw, and logs a change so a run says which of
/// the three the platform reported.
fn record(source: ScrollSource) {
    LAST.with(|last| {
        if last.get().map(|(previous, _)| previous) != Some(source) {
            log::debug!(target: "shell", "scroll: input is now {source:?}");
        }
        last.set(Some((source, Instant::now())));
    });
}

/// The macOS classifier: an `NSEvent` observer for scroll-wheel events.
///
/// # Why an observer and not a swizzle
///
/// `-[NSEvent addLocalMonitorForEventsMatchingMask:handler:]` is the supported
/// way to see events on their way to a view: the handler runs while AppKit
/// dispatches, before the event reaches the window, and it returns the event
/// untouched. A local monitor needs no permission (unlike a global one), and
/// nothing is replaced — unlike the `NSThemeFrame` overrides in
/// `traffic_lights.rs`, which had no such hook to use.
///
/// # What it reads
///
/// `hasPreciseScrollingDeltas` is AppKit saying the event came from a surface
/// with more resolution than a notched wheel — a trackpad, or a Magic Mouse,
/// which behaves like one and belongs on the same path. `momentumPhase` is
/// non-zero exactly for the events macOS synthesizes after the fingers left.
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

    /// Starts observing scroll-wheel events app-wide.
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

    /// Wheel, trackpad or the trackpad's momentum.
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
