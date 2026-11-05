// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Routes AppKit's terminate requests (`⌘Q`, the application menu's Quit, a
//! system logout) through the window's own close flow: `applicationShouldTerminate:`
//! is added to winit's delegate to cancel and replay the quit as a close, so the
//! "a task is running" confirm still runs. Both the delegate and the added method
//! run on the main thread; `install` runs from `main` before `ui.run`.

use core::ffi::{CStr, c_char, c_void};
use core::ptr::NonNull;
use std::cell::RefCell;

use block2::RcBlock;
use objc2::ffi;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSNotification, NSNotificationCenter, ns_string};

/// A borrowed Objective-C object.
type Id = *mut AnyObject;
/// An Objective-C selector.
type SelPtr = *const ffi::objc_selector;

// `applicationShouldTerminate:`'s implementation, stored for the runtime method
// below: a C function cannot capture the shell handle, so it reads it here.
//
// Thread-local rather than a global: the method only runs on the main thread,
// and the handle is not `Sync`.
thread_local! {
    static REQUEST_CLOSE: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Registers `request_close` as the answer to AppKit's terminate requests, and
/// `run_exit_work` as the exit-time work.
///
/// `request_close` runs on the main thread and is expected to hand the request
/// to the shell's event loop and return immediately, so a slow confirm does not
/// block the delegate call that is waiting for its reply.
pub(super) fn install(request_close: impl Fn() + 'static, run_exit_work: impl Fn() + 'static) {
    REQUEST_CLOSE.with(|slot| *slot.borrow_mut() = Some(Box::new(request_close)));

    // AppKit posts this on its way out, and the terminate request is cancelled
    // and replayed as a close, so this is reached only when the app really is
    // terminating — the last chance to run the exit-time work.
    let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
        run_exit_work();
    });

    // The notification center keeps the returned observer alive; there is one
    // app-lifetime observation, so there is nothing to remove it later.
    unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(ns_string!("NSApplicationWillTerminateNotification")),
            None,
            None,
            &block,
        );
    }

    install_should_terminate();
}

fn install_should_terminate() {
    const SELECTOR: &CStr = c"applicationShouldTerminate:";

    // SAFETY: every step goes through the Objective-C runtime and runs on the
    // main thread, where the delegate was set.
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let delegate: Id = msg_send![app, delegate];
        if delegate.is_null() {
            log::warn!(target: "shell", "terminate: no application delegate to answer for");
            return;
        }

        let class =
            ffi::object_getClass(delegate.cast::<ffi::objc_object>()) as *mut ffi::objc_class;
        let added = (ffi::class_addMethod(
            class,
            ffi::sel_registerName(SELECTOR.as_ptr()),
            as_imp(application_should_terminate as *const () as *const c_void),
            method_types(SELECTOR),
        ) as i8)
            != 0;

        // The delegate is not expected to implement this, so a failure means a
        // future winit took the selector over. Say so loudly rather than
        // silently replacing an implementation whose contract we do not know.
        if added {
            log::debug!(
                target: "shell",
                "terminate: routing the platform Quit through the close flow"
            );
        } else {
            log::warn!(
                target: "shell",
                "terminate: the application delegate already answers applicationShouldTerminate:"
            );
        }
    }
}

/// `-[<delegate> applicationShouldTerminate:]`.
///
/// Cancels the terminate and asks the shell to close instead, so the request
/// takes the same path as `⌘W`: confirm first when a task is running, close
/// outright otherwise.
unsafe extern "C" fn application_should_terminate(_this: Id, _cmd: SelPtr, _sender: Id) -> usize {
    log::debug!(
        target: "shell",
        "terminate: the platform asked to quit; replaying it through the close flow"
    );
    let mut asked = false;
    REQUEST_CLOSE.with(|slot| {
        if let Some(request) = slot.borrow().as_ref() {
            asked = true;
            request();
        }
    });
    if !asked {
        // No handler installed, yet `NSTerminateCancel` still goes back — so the
        // quit is refused with nothing on screen saying why.
        log::warn!(
            target: "shell",
            "terminate: the platform asked to quit and nothing is listening, so the quit \
             is being refused"
        );
    }
    // `NSTerminateCancel`. (`NSTerminateNow` is 1.)
    0
}

/// The runtime type encoding for `selector` on the `NSApplicationDelegate`
/// protocol.
///
/// Read from the protocol rather than hardcoded, so the encoding cannot drift
/// from the SDK the app is linked against. Falls back to the `NSUInteger`
/// spelling if the protocol is not registered.
fn method_types(selector: &CStr) -> *const c_char {
    // SAFETY: the protocol and selector names are NUL-terminated, and the
    // returned pointer either comes from the runtime or is a static literal.
    unsafe {
        let protocol = ffi::objc_getProtocol(c"NSApplicationDelegate".as_ptr());
        if !protocol.is_null() {
            let types = ffi::protocol_getMethodDescription(
                protocol,
                ffi::sel_registerName(selector.as_ptr()),
                ffi::NO,
                ffi::YES,
            )
            .types;
            if !types.is_null() {
                return types;
            }
        }
    }
    // A hardcoded encoding is a guess: if the protocol is not registered — an
    // unusual runtime, or a future SDK — this is the encoding AppKit will decode
    // the selector through, and a wrong one fails as an unrecognised selector with
    // nothing to say which of the two paths produced it.
    log::debug!(
        target: "shell",
        "terminate: no protocol method description for {selector:?}; using the \
         NSUInteger encoding"
    );
    c"Q@:@".as_ptr()
}

/// Re-types a function pointer for the untyped `IMP` the runtime stores.
fn as_imp(implementation: *const c_void) -> ffi::IMP {
    if implementation.is_null() {
        return None;
    }
    // SAFETY: a function pointer and `*const c_void` are both pointer-sized, and
    // `implementation` is always the address of an `extern "C"` function.
    Some(unsafe { core::mem::transmute::<*const c_void, unsafe extern "C" fn()>(implementation) })
}
