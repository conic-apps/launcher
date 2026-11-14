// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Keeps the native AppKit traffic lights aligned with the custom title bar by
//! answering the metrics AppKit asks for, instead of fighting its layout with
//! frame writes.
//!
//! Fullscreen hides the buttons; [`watch_fullscreen`] reports that to the UI.
//! [`install_titlebar_guard`] removes AppKit's double-click zoom so the custom
//! bar can handle it.
//!
//! The metric selectors are private API, so [`install()`] self-checks and falls
//! back to writing frames if they stop working. Set
//! `CONIC_FORCE_TRAFFIC_LIGHT_FALLBACK=1` to exercise that path.

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_void};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use core::ptr::NonNull;

use block2::RcBlock;
use objc2::ffi;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{class, msg_send};
use objc2_foundation::{
    CGPoint, CGRect, NSNotification, NSNotificationCenter, NSString, ns_string,
};

/// Height of the custom title bar in `ui/components/title-bar.slint`; AppKit
/// centres the buttons inside it.
const TITLEBAR_HEIGHT: f64 = 44.0;

const WIDGET_INSET_X: f64 = 16.0;

/// `NSWindowStyleMaskTitled`.
const STYLE_MASK_TITLED: usize = 1 << 0;
/// `NSWindowStyleMaskFullScreen`.
const STYLE_MASK_FULLSCREEN: usize = 1 << 14;

/// `NSWindowCloseButton`; miniaturize and zoom follow it.
const CLOSE_BUTTON: usize = 0;

// Call counters for the self-check diagnostics.
static TITLEBAR_HEIGHT_CALLS: AtomicUsize = AtomicUsize::new(0);
static WIDGET_INSET_CALLS: AtomicUsize = AtomicUsize::new(0);
static CENTER_TRAFFIC_LIGHTS_CALLS: AtomicUsize = AtomicUsize::new(0);
static STYLE_MASK_CALLS: AtomicUsize = AtomicUsize::new(0);

const EXPECTED_CENTER_Y: f64 = TITLEBAR_HEIGHT / 2.0;

/// AppKit rounds to the backing store.
const CHECK_TOLERANCE: f64 = 1.5;
const CHECK_TIMEOUT: Duration = Duration::from_millis(1000);

type Id = *mut AnyObject;
type SelPtr = *const ffi::objc_selector;

type FrameViewClassImp =
    unsafe extern "C" fn(*const ffi::objc_class, SelPtr, usize) -> *mut ffi::objc_class;
type TitlebarHeightImp = unsafe extern "C" fn(Id, SelPtr) -> f64;
type SetStyleMaskImp = unsafe extern "C" fn(Id, SelPtr, usize);

/// AppKit's original implementation, kept for untitled windows.
static ORIGINAL_FRAME_VIEW_CLASS_IMP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
/// The `ConicThemeFrame` class, or null while the metric route is unavailable.
static THEME_FRAME_CLASS: AtomicPtr<ffi::objc_class> = AtomicPtr::new(ptr::null_mut());
static SUPER_TITLEBAR_HEIGHT_IMP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SUPER_SET_STYLE_MASK_IMP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

/// Used only as an associated-object key, so the value is never read.
static STYLE_MASK_KEY: u8 = 0;

/// Installs the traffic-light mechanism. Must run before the first `NSWindow`
/// exists.
pub fn install() {
    install_titlebar_guard();
    if std::env::var_os("CONIC_FORCE_TRAFFIC_LIGHT_FALLBACK").is_some() {
        log::warn!(
            target: "shell",
            "traffic lights: forced onto the fallback by CONIC_FORCE_TRAFFIC_LIGHT_FALLBACK"
        );
        install_fallback();
        return;
    }

    if !install_theme_frame_class() {
        log::warn!(
            target: "shell",
            "traffic lights: NSThemeFrame is unavailable, using the frame-writing fallback"
        );
        install_fallback();
        return;
    }

    log::debug!(
        target: "shell",
        "traffic lights: metric route installed (titlebar {TITLEBAR_HEIGHT}px, \
         inset {WIDGET_INSET_X}px, buttons centred at {EXPECTED_CENTER_Y}px)"
    );
    arm_self_check();
}

/// Keeps the UI's `window-fullscreen` flag in step with the window, so the title
/// bar knows whether to leave room for the traffic lights.
pub fn watch_fullscreen(ui: &crate::slint_backend::App) {
    use slint::ComponentHandle;

    let weak = ui.as_weak();
    let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        let notification = unsafe { notification.as_ref() };
        let Some(window) = (unsafe { notification.object() }) else {
            return;
        };
        let window = &*window as *const AnyObject as Id;
        let fullscreen = unsafe { is_fullscreen_window(window) };
        log::debug!(target: "shell", "traffic lights: the window is now fullscreen={fullscreen}");
        if let Some(ui) = weak.upgrade() {
            ui.set_window_fullscreen(fullscreen);
        }
    });

    observe(
        &[
            ns_string!("NSWindowDidEnterFullScreenNotification"),
            ns_string!("NSWindowDidExitFullScreenNotification"),
        ],
        &block,
    );
    log::debug!(target: "shell", "traffic lights: watching the fullscreen state");
}

/// Registers `ConicThemeFrame` and makes `NSWindow` hand it out for titled
/// windows. Returns `false`, changing nothing, if the private API is missing.
fn install_theme_frame_class() -> bool {
    // SAFETY: runtime calls on the main thread before any window exists.
    unsafe {
        let theme_frame = ffi::objc_getClass(c"NSThemeFrame".as_ptr());
        if theme_frame.is_null() {
            return false;
        }

        let Some(super_titlebar_height) = method_implementation(theme_frame, c"_titlebarHeight")
        else {
            return false;
        };
        let Some(super_set_style_mask) = method_implementation(theme_frame, c"setStyleMask:")
        else {
            return false;
        };

        let subclass = ffi::objc_allocateClassPair(theme_frame, c"ConicThemeFrame".as_ptr(), 0);
        if subclass.is_null() {
            log::warn!(target: "shell", "traffic lights: ConicThemeFrame is already registered");
            return false;
        }

        let added = [
            add_method(
                subclass,
                c"_titlebarHeight",
                titlebar_height as *const () as *const c_void,
                c"d@:",
            ),
            add_method(
                subclass,
                c"_minXTitlebarWidgetInset",
                min_x_titlebar_widget_inset as *const () as *const c_void,
                c"d@:",
            ),
            add_method(
                subclass,
                c"_shouldCenterTrafficLights",
                should_center_traffic_lights as *const () as *const c_void,
                c"c@:",
            ),
            add_method(
                subclass,
                c"setStyleMask:",
                set_style_mask_override as *const () as *const c_void,
                c"v@:Q",
            ),
        ];
        if added.contains(&false) {
            ffi::objc_disposeClassPair(subclass);
            log::warn!(target: "shell", "traffic lights: could not add the metric overrides");
            return false;
        }

        // winit's `WinitWindow` does not override this, so taking it over on
        // `NSWindow` reaches our window.
        let style_mask_sel = ffi::sel_registerName(c"frameViewClassForStyleMask:".as_ptr());
        let Some(factory) =
            (ffi::class_getClassMethod(ffi::objc_getClass(c"NSWindow".as_ptr()), style_mask_sel))
                .as_ref()
        else {
            ffi::objc_disposeClassPair(subclass);
            log::warn!(
                target: "shell",
                "traffic lights: +[NSWindow frameViewClassForStyleMask:] is missing"
            );
            return false;
        };
        let Some(original) = ffi::method_getImplementation(factory) else {
            ffi::objc_disposeClassPair(subclass);
            log::warn!(
                target: "shell",
                "traffic lights: +[NSWindow frameViewClassForStyleMask:] has no \
                 implementation"
            );
            return false;
        };

        // Committing: from here on there is nothing left that can fail.
        ffi::objc_registerClassPair(subclass);
        THEME_FRAME_CLASS.store(subclass, Ordering::Relaxed);
        SUPER_TITLEBAR_HEIGHT_IMP.store(imp_to_ptr(Some(super_titlebar_height)), Ordering::Relaxed);
        SUPER_SET_STYLE_MASK_IMP.store(imp_to_ptr(Some(super_set_style_mask)), Ordering::Relaxed);
        ORIGINAL_FRAME_VIEW_CLASS_IMP.store(imp_to_ptr(Some(original)), Ordering::Relaxed);
        ffi::method_setImplementation(
            factory,
            as_imp(frame_view_class as *const () as *const c_void),
        );
        true
    }
}

/// Registers `implementation` for `selector` on `cls`; `types` is the Objective-C
/// type encoding and must match the function signature.
fn add_method(
    cls: *mut ffi::objc_class,
    selector: &CStr,
    implementation: *const c_void,
    types: &CStr,
) -> bool {
    // SAFETY: `cls` is a class pair that has not been registered yet, and both
    // the selector and the encoding are NUL-terminated C strings.
    unsafe {
        (ffi::class_addMethod(
            cls,
            ffi::sel_registerName(selector.as_ptr()),
            as_imp(implementation),
            types.as_ptr(),
        ) as i8)
            != 0
    }
}

/// Re-types a method implementation for the runtime's untyped `IMP`.
fn as_imp(implementation: *const c_void) -> ffi::IMP {
    if implementation.is_null() {
        return None;
    }
    // SAFETY: a function pointer and `*const c_void` are both pointer-sized,
    // and `implementation` is always the address of an `extern "C"` function.
    Some(unsafe { std::mem::transmute::<*const c_void, unsafe extern "C" fn()>(implementation) })
}

/// Removes AppKit's double-click zoom so the custom title bar can handle it
/// before Slint sees the press. Dragging is untouched.
fn install_titlebar_guard() {
    // SAFETY: runtime calls on the main thread before the window exists.
    unsafe {
        let window = ffi::objc_getClass(c"NSWindow".as_ptr());
        if window.is_null() {
            log::warn!(target: "shell", "titlebar guard: NSWindow is unavailable");
            return;
        }

        let zoom = set_method_implementation(
            window,
            c"zoom:",
            suppress_titlebar_zoom as *const () as *const c_void,
        );
        let perform_zoom = set_method_implementation(
            window,
            c"performZoom:",
            suppress_titlebar_zoom as *const () as *const c_void,
        );

        if zoom && perform_zoom {
            log::debug!(target: "shell", "titlebar guard: native double-click zoom suppressed");
        } else {
            log::warn!(
                target: "shell",
                "titlebar guard: incomplete (zoom {zoom}, performZoom {perform_zoom}); a \
                 double-click on the title bar may still maximise the window"
            );
        }
    }
}

/// `-[NSWindow zoom:]` / `-performZoom:`: the native title bar's double-click.
unsafe extern "C" fn suppress_titlebar_zoom(_this: Id, _cmd: SelPtr, _sender: Id) {}

/// `-[NSThemeFrame _titlebarHeight]`.
unsafe extern "C" fn titlebar_height(this: Id, cmd: SelPtr) -> f64 {
    TITLEBAR_HEIGHT_CALLS.fetch_add(1, Ordering::Relaxed);
    // Fullscreen keeps AppKit's height. The live style mask is unreliable
    // mid-transition, so the recorded bit is used instead.
    let super_imp = SUPER_TITLEBAR_HEIGHT_IMP.load(Ordering::Relaxed);
    if !super_imp.is_null() && unsafe { is_fullscreen(this) } {
        let original: TitlebarHeightImp = unsafe { std::mem::transmute(super_imp) };
        return unsafe { original(this, cmd) };
    }
    TITLEBAR_HEIGHT
}

/// `-[NSThemeFrame _minXTitlebarWidgetInset]`.
unsafe extern "C" fn min_x_titlebar_widget_inset(_this: Id, _cmd: SelPtr) -> f64 {
    WIDGET_INSET_CALLS.fetch_add(1, Ordering::Relaxed);
    WIDGET_INSET_X
}

/// `-[NSThemeFrame _shouldCenterTrafficLights]`: `YES`.
unsafe extern "C" fn should_center_traffic_lights(_this: Id, _cmd: SelPtr) -> i8 {
    CENTER_TRAFFIC_LIGHTS_CALLS.fetch_add(1, Ordering::Relaxed);
    1
}

/// `-[NSThemeFrame setStyleMask:]`: record the fullscreen bit, then forward.
unsafe extern "C" fn set_style_mask_override(this: Id, cmd: SelPtr, style_mask: usize) {
    STYLE_MASK_CALLS.fetch_add(1, Ordering::Relaxed);
    let number: Id = unsafe { msg_send![class!(NSNumber), numberWithUnsignedInteger: style_mask] };
    unsafe {
        ffi::objc_setAssociatedObject(
            this.cast::<ffi::objc_object>(),
            ptr::addr_of!(STYLE_MASK_KEY).cast::<c_void>(),
            number.cast::<ffi::objc_object>(),
            ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
        );
    }

    let super_imp = SUPER_SET_STYLE_MASK_IMP.load(Ordering::Relaxed);
    if !super_imp.is_null() {
        let original: SetStyleMaskImp = unsafe { std::mem::transmute(super_imp) };
        unsafe { original(this, cmd, style_mask) };
    }
}

/// Replacement for `+[NSWindow frameViewClassForStyleMask:]`.
unsafe extern "C" fn frame_view_class(
    cls: *const ffi::objc_class,
    cmd: SelPtr,
    style_mask: usize,
) -> *mut ffi::objc_class {
    let ours = THEME_FRAME_CLASS.load(Ordering::Relaxed);
    if style_mask & STYLE_MASK_TITLED != 0 && !ours.is_null() {
        return ours;
    }

    let original = ORIGINAL_FRAME_VIEW_CLASS_IMP.load(Ordering::Relaxed);
    if original.is_null() {
        return ptr::null_mut();
    }
    let original: FrameViewClassImp = unsafe { std::mem::transmute(original) };
    unsafe { original(cls, cmd, style_mask) }
}

unsafe fn is_fullscreen(frame: Id) -> bool {
    let recorded = unsafe {
        ffi::objc_getAssociatedObject(
            frame.cast::<ffi::objc_object>(),
            ptr::addr_of!(STYLE_MASK_KEY).cast::<c_void>(),
        )
    };
    if recorded.is_null() {
        return false;
    }
    let style_mask: usize = unsafe {
        msg_send![
            recorded.cast::<AnyObject>().cast_mut(),
            unsignedIntegerValue
        ]
    };
    style_mask & STYLE_MASK_FULLSCREEN != 0
}

/// Replaces `name` on `cls`; `false` if the selector is missing.
unsafe fn set_method_implementation(
    cls: *const ffi::objc_class,
    name: &CStr,
    implementation: *const c_void,
) -> bool {
    let sel = unsafe { ffi::sel_registerName(name.as_ptr()) };
    let method = unsafe { ffi::class_getInstanceMethod(cls, sel) };
    let Some(method) = (unsafe { method.as_ref() }) else {
        return false;
    };
    unsafe { ffi::method_setImplementation(method, as_imp(implementation)) };
    true
}

/// A method's implementation, or `None` if the selector is gone.
unsafe fn method_implementation(cls: *const ffi::objc_class, name: &CStr) -> ffi::IMP {
    let method = unsafe { ffi::class_getInstanceMethod(cls, ffi::sel_registerName(name.as_ptr())) };
    if method.is_null() {
        log::warn!(
            target: "shell",
            "traffic lights: {} is missing from this macOS",
            name.to_string_lossy()
        );
        return None;
    }
    unsafe { ffi::method_getImplementation(method) }
}

fn imp_to_ptr(imp: ffi::IMP) -> *mut c_void {
    match imp {
        Some(function) => function as *const () as *mut c_void,
        None => ptr::null_mut(),
    }
}

/// Arms the one-shot check that decides whether the metric route works.
fn arm_self_check() {
    let started = Rc::new(Cell::new(None::<Instant>));
    let settled = Rc::new(Cell::new(false));

    let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        if settled.get() {
            return;
        }
        let notification = unsafe { notification.as_ref() };
        let Some(window) = (unsafe { notification.object() }) else {
            return;
        };
        let window = &*window as *const AnyObject as Id;
        if !unsafe { window_visible(window) } || unsafe { is_fullscreen_window(window) } {
            return;
        }

        let now = Instant::now();
        match started.get() {
            None => started.set(Some(now)),
            Some(first_seen) if now.duration_since(first_seen) >= CHECK_TIMEOUT => {
                settled.set(true);
                settle_failed(window);
                return;
            }
            Some(_) => {}
        }

        let Some(observed) = (unsafe { close_button_position(window) }) else {
            return;
        };
        if !unsafe { uses_conic_theme_frame(window) } {
            settled.set(true);
            log::warn!(
                target: "shell",
                "traffic lights: the window kept AppKit's frame view, falling back"
            );
            settle_failed(window);
            return;
        }
        if (observed.x - WIDGET_INSET_X).abs() > CHECK_TOLERANCE
            || (observed.center_y - EXPECTED_CENTER_Y).abs() > CHECK_TOLERANCE
        {
            // AppKit may not have laid the titlebar out yet; wait for the timeout.
            return;
        }

        settled.set(true);
        log::debug!(
            target: "shell",
            "traffic lights: metric route verified (close button at x {:.1}, centre {:.1}, \
             left edges {})",
            observed.x,
            observed.center_y,
            unsafe { button_left_edges(window) }
        );
    });

    observe(&[ns_string!("NSWindowDidUpdateNotification")], &block);
}

fn settle_failed(window: Id) {
    unsafe { log_window_state(window) };
    if let Some(observed) = unsafe { close_button_position(window) } {
        log::warn!(
            target: "shell",
            "traffic lights: metric route did not take (close button at x {:.1}, centre {:.1}, \
             wanted x {WIDGET_INSET_X}, centre {EXPECTED_CENTER_Y}), falling back",
            observed.x,
            observed.center_y
        );
    }
    install_fallback();
}

/// Logs the window's titlebar chain at debug level, when the metric route is
/// about to give up.
unsafe fn log_window_state(window: Id) {
    let theme_frame = unsafe { theme_frame(window) };
    let factory: *const AnyClass =
        unsafe { msg_send![class!(NSWindow), frameViewClassForStyleMask: STYLE_MASK_TITLED] };
    let factory = if factory.is_null() {
        "<nil>".to_owned()
    } else {
        unsafe { (*factory).name() }.to_owned()
    };
    let frame: CGRect = unsafe { msg_send![window, frame] };
    log::debug!(
        target: "shell",
        "traffic lights: window {} ({:.0}x{:.0}), frame view {}, \
         +frameViewClassForStyleMask: hands out {factory}, ConicThemeFrame registered: {}",
        unsafe { class_name(window) },
        frame.size.width,
        frame.size.height,
        unsafe { class_name(theme_frame) },
        !THEME_FRAME_CLASS.load(Ordering::Relaxed).is_null(),
    );
    log::debug!(
        target: "shell",
        "traffic lights: AppKit asked for _titlebarHeight {}x, _minXTitlebarWidgetInset {}x, \
         _shouldCenterTrafficLights {}x, setStyleMask: {}x",
        TITLEBAR_HEIGHT_CALLS.load(Ordering::Relaxed),
        WIDGET_INSET_CALLS.load(Ordering::Relaxed),
        CENTER_TRAFFIC_LIGHTS_CALLS.load(Ordering::Relaxed),
        STYLE_MASK_CALLS.load(Ordering::Relaxed),
    );
    if let Some(container) = unsafe { titlebar_container(window) } {
        let container_frame: CGRect = unsafe { msg_send![container, frame] };
        log::debug!(
            target: "shell",
            "traffic lights: container {} at ({:.1}, {:.1}) {:.0}x{:.0}",
            unsafe { class_name(container) },
            container_frame.origin.x,
            container_frame.origin.y,
            container_frame.size.width,
            container_frame.size.height,
        );
    }

    // The chain from the frame view down to the close button, in the frame
    // view's coordinates: this is what says where AppKit put the pieces.
    let close: Id = unsafe { msg_send![window, standardWindowButton: CLOSE_BUTTON] };
    let mut view = close;
    while !view.is_null() && view != theme_frame {
        if let Some(position) = unsafe { position_in(view, theme_frame) } {
            log::debug!(
                target: "shell",
                "traffic lights:   {} at ({:.1}, {:.1}) {:.0}x{:.0}",
                unsafe { class_name(view) },
                position.origin.x,
                position.origin.y,
                position.size.width,
                position.size.height,
            );
        }
        view = unsafe { msg_send![view, superview] };
    }
}

unsafe fn position_in(view: Id, target: Id) -> Option<CGRect> {
    unsafe {
        let superview: Id = msg_send![view, superview];
        if superview.is_null() {
            return None;
        }
        let frame: CGRect = msg_send![view, frame];
        Some(msg_send![superview, convertRect: frame, toView: target])
    }
}

unsafe fn class_name(object: Id) -> String {
    if object.is_null() {
        return "<nil>".to_owned();
    }
    let class = unsafe { ffi::object_getClass(object.cast::<ffi::objc_object>()) };
    if class.is_null() {
        return "<nil>".to_owned();
    }
    let name = unsafe { ffi::class_getName(class) };
    if name.is_null() {
        return "<unnamed>".to_owned();
    }
    unsafe { CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned()
}

/// The close button's left edge and its vertical centre, both measured from the
/// window's top edge.
struct Position {
    x: f64,
    center_y: f64,
}

unsafe fn close_button_position(window: Id) -> Option<Position> {
    let close: Id = unsafe { msg_send![window, standardWindowButton: CLOSE_BUTTON] };
    if close.is_null() {
        return None;
    }
    let in_window = unsafe { position_in_window(close) }?;
    let window_frame: CGRect = unsafe { msg_send![window, frame] };
    Some(Position {
        x: in_window.origin.x,
        // Window coordinates grow upwards, the design is specified downwards.
        center_y: window_frame.size.height - (in_window.origin.y + in_window.size.height / 2.0),
    })
}

/// A view's frame in the window's coordinates, converted on the superview: a
/// frame is expressed in its superview's coordinates, so converting through the
/// view itself counts the offset twice.
unsafe fn position_in_window(view: Id) -> Option<CGRect> {
    unsafe {
        let superview: Id = msg_send![view, superview];
        if superview.is_null() {
            return None;
        }
        let frame: CGRect = msg_send![view, frame];
        Some(msg_send![
            superview,
            convertRect: frame,
            toView: ptr::null_mut::<AnyObject>()
        ])
    }
}

/// The close/miniaturize/zoom left edges, for the verification log.
unsafe fn button_left_edges(window: Id) -> String {
    let mut edges = Vec::with_capacity(3);
    for index in 0..3usize {
        let button: Id = unsafe { msg_send![window, standardWindowButton: CLOSE_BUTTON + index] };
        match unsafe { position_in_window(button) } {
            Some(position) => edges.push(format!("{:.1}", position.origin.x)),
            None => edges.push("?".to_owned()),
        }
    }
    edges.join(", ")
}

/// Whether the window's frame view is `ConicThemeFrame`. The superclass chain is
/// walked, not the class pointer: KVO hands out a dynamic subclass
/// (`NSKVONotifying_ConicThemeFrame`) that still inherits the overrides.
unsafe fn uses_conic_theme_frame(window: Id) -> bool {
    let frame_view = unsafe { theme_frame(window) };
    if frame_view.is_null() {
        return false;
    }
    unsafe {
        let class = ffi::object_getClass(frame_view.cast::<ffi::objc_object>());
        inherits_from(
            class,
            THEME_FRAME_CLASS.load(Ordering::Relaxed).cast_const(),
        )
    }
}

unsafe fn inherits_from(mut class: *const ffi::objc_class, target: *const ffi::objc_class) -> bool {
    while !class.is_null() {
        if class == target {
            return true;
        }
        class = unsafe { ffi::class_getSuperclass(class) };
    }
    false
}

// The frame-writing fallback, used when the metric route is unavailable. The
// geometry is derived from AppKit rather than hardcoded, and the container is
// never hidden — hiding it lets AppKit re-lay the titlebar out from under us.

/// AppKit's own button geometry, read before anything is moved.
struct Metrics {
    /// How far the buttons' bottom edge sits above the container's bottom edge.
    ///
    /// AppKit's own value, so the fallback does not have to hardcode the 9pt.
    pad_below: f64,
    /// Distance between the left edges of two neighbouring buttons.
    pitch: f64,
    button_height: f64,
}

impl Metrics {
    /// The container height that puts the buttons' centre at
    /// [`EXPECTED_CENTER_Y`].
    fn container_height(&self) -> f64 {
        EXPECTED_CENTER_Y + self.button_height / 2.0 + self.pad_below
    }
}

/// Installs the frame-writing fallback: re-apply the geometry whenever AppKit
/// has had a chance to reset it.
fn install_fallback() {
    let metrics = Rc::new(RefCell::new(None::<Metrics>));
    let reported = Rc::new(Cell::new(false));

    let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        let notification = unsafe { notification.as_ref() };
        let Some(window) = (unsafe { notification.object() }) else {
            return;
        };
        let window = &*window as *const AnyObject as Id;
        // Fullscreen owns the title bar; AppKit re-shows the buttons on exit.
        if unsafe { is_fullscreen_window(window) } {
            return;
        }
        let mut cached = metrics.borrow_mut();
        if cached.is_none() {
            // SAFETY: called on the main thread from a notification, and before
            // the first `apply_metrics` — so this still describes AppKit's own layout.
            match unsafe { measure(window) } {
                Some(measured) => *cached = Some(measured),
                // No buttons yet (or no frame view): try again on the next event
                // rather than caching a bogus geometry.
                None => return,
            }
        }
        if let Some(measured) = cached.as_ref() {
            unsafe { apply_metrics(window, measured) };
            if !reported.get() {
                reported.set(true);
                // One line per run, so the fallback's result is checkable the
                // same way the metric route's is.
                log::debug!(
                    target: "shell",
                    "traffic lights: fallback geometry — button {:.0}pt, {:.0}pt above the \
                     container bottom, {:.0}pt pitch, container {:.0}pt; buttons now at {}",
                    measured.button_height,
                    measured.pad_below,
                    measured.pitch,
                    measured.container_height(),
                    unsafe { button_left_edges(window) },
                );
            }
        }
    });

    observe(
        &[
            ns_string!("NSWindowDidUpdateNotification"),
            ns_string!("NSWindowDidResizeNotification"),
            ns_string!("NSWindowDidBecomeKeyNotification"),
            ns_string!("NSWindowDidResignKeyNotification"),
            ns_string!("NSWindowDidBecomeMainNotification"),
            ns_string!("NSWindowDidResignMainNotification"),
            ns_string!("NSWindowDidOrderOnScreenNotification"),
            ns_string!("NSWindowDidOrderOffScreenNotification"),
            ns_string!("NSWindowDidChangeScreenNotification"),
            ns_string!("NSWindowDidChangeBackingPropertiesNotification"),
            ns_string!("NSWindowWillEnterFullScreenNotification"),
            ns_string!("NSWindowDidExitFullScreenNotification"),
            ns_string!("NSWindowDidFailToEnterFullScreenNotification"),
        ],
        &block,
    );
    log::warn!(target: "shell", "traffic lights: frame-writing fallback installed");
}

/// Reads AppKit's own geometry, so nothing has to be hardcoded.
unsafe fn measure(window: Id) -> Option<Metrics> {
    let container = unsafe { titlebar_container(window) }?;
    let close: Id = unsafe { msg_send![window, standardWindowButton: CLOSE_BUTTON] };
    let miniaturize: Id = unsafe { msg_send![window, standardWindowButton: CLOSE_BUTTON + 1] };
    if close.is_null() || miniaturize.is_null() {
        return None;
    }

    unsafe {
        let close_in_window = position_in_window(close)?;
        let miniaturize_in_window = position_in_window(miniaturize)?;
        let container_in_window = position_in_window(container)?;
        let close_frame: CGRect = msg_send![close, frame];
        Some(Metrics {
            pad_below: close_in_window.origin.y - container_in_window.origin.y,
            pitch: miniaturize_in_window.origin.x - close_in_window.origin.x,
            button_height: close_frame.size.height,
        })
    }
}

unsafe fn apply_metrics(window: Id, metrics: &Metrics) {
    let Some(container) = (unsafe { titlebar_container(window) }) else {
        return;
    };

    unsafe {
        let window_frame: CGRect = msg_send![window, frame];
        let mut container_frame: CGRect = msg_send![container, frame];
        let height = metrics.container_height();
        let origin_y = window_frame.size.height - height;
        if container_frame.size.height != height || container_frame.origin.y != origin_y {
            container_frame.size.height = height;
            container_frame.origin.y = origin_y;
            let _: () = msg_send![container, setFrame: container_frame];
        }

        for index in 0..3usize {
            let button: Id = msg_send![window, standardWindowButton: CLOSE_BUTTON + index];
            if button.is_null() {
                continue;
            }
            let frame: CGRect = msg_send![button, frame];
            let x = WIDGET_INSET_X + index as f64 * metrics.pitch;
            if frame.origin.x != x {
                let _: () = msg_send![button, setFrameOrigin: CGPoint::new(x, frame.origin.y)];
            }
        }
    }
}

/// Registers `block` for every notification in `names` on the default center.
/// The tokens and the block are leaked: these observers live as long as the
/// process does.
fn observe(names: &[&NSString], block: &block2::Block<dyn Fn(NonNull<NSNotification>)>) {
    unsafe {
        let center = NSNotificationCenter::defaultCenter();
        for name in names {
            center.addObserverForName_object_queue_usingBlock(Some(name), None, None, block);
        }
    }
}

unsafe fn window_visible(window: Id) -> bool {
    unsafe { msg_send![window, isVisible] }
}

/// Whether the window is in (or entering) fullscreen.
unsafe fn is_fullscreen_window(window: Id) -> bool {
    let style_mask: usize = unsafe { msg_send![window, styleMask] };
    style_mask & STYLE_MASK_FULLSCREEN != 0
}

/// The buttons' container (`NSTitlebarContainerView`): the close button's
/// superview's superview.
unsafe fn titlebar_container(window: Id) -> Option<Id> {
    unsafe {
        let close: Id = msg_send![window, standardWindowButton: CLOSE_BUTTON];
        if close.is_null() {
            return None;
        }
        let titlebar: Id = msg_send![close, superview];
        if titlebar.is_null() {
            return None;
        }
        let container: Id = msg_send![titlebar, superview];
        (!container.is_null()).then_some(container)
    }
}

/// The window's frame view (`NSThemeFrame` and friends), i.e. the frame the
/// buttons' coordinates are measured against.
unsafe fn theme_frame(window: Id) -> Id {
    unsafe {
        let content_view: Id = msg_send![window, contentView];
        if content_view.is_null() {
            return ptr::null_mut();
        }
        msg_send![content_view, superview]
    }
}
