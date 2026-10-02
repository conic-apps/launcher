// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Sets the macOS Dock / task-switcher icon.
//!
//! The Slint `Window.icon` binding can't reach it: the winit backend forwards
//! that to `winit::Window::set_window_icon`, which is a no-op on macOS. Instead
//! the embedded PNG is loaded into an `NSImage` and handed to the shared
//! `NSApplication`. Windows and Linux use the `icon:` binding in `app.slint`.

/// Defers [`set`] until the app has finished launching.
///
/// `setApplicationIconImage:` only sticks after `applicationDidFinishLaunching:`
/// (which AppKit runs inside `ui.run()`); setting it before that is discarded
/// when AppKit initializes the app icon during launch.
pub(super) fn install() {
    use core::ptr::NonNull;
    use objc2_foundation::{NSNotification, NSNotificationCenter, ns_string};

    let block = block2::RcBlock::new(move |_notification: NonNull<NSNotification>| {
        set();
    });

    unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(ns_string!("NSApplicationDidFinishLaunchingNotification")),
            None,
            None,
            &block,
        );
    }
}

/// Hands the embedded PNG to the shared `NSApplication`.
fn set() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    const ICON: &[u8] = include_bytes!("../../../ui/assets/images/app-icon.png");

    unsafe {
        let data: *mut AnyObject = msg_send![
            class!(NSData),
            dataWithBytes: ICON.as_ptr() as *const core::ffi::c_void,
            length: ICON.len()
        ];
        let image: *mut AnyObject = msg_send![class!(NSImage), alloc];
        let image: *mut AnyObject = msg_send![image, initWithData: data];
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, setApplicationIconImage: image];
        // `setApplicationIconImage:` retains the image; balance our alloc/init.
        let _: () = msg_send![image, release];
    }
}
