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
        // The observer token is dropped on purpose (the notification centre keeps
        // the block alive), so a nil return cannot be acted on — but a *missing*
        // registration can be recorded, and it means the icon is never set at all.
        // `Retained` is non-null by construction, so the only observable signal is
        // whether the call returned anything; AppKit refusing is logged below by
        // `set` itself, which is where the failure actually shows up.
        let _observer = NSNotificationCenter::defaultCenter()
            .addObserverForName_object_queue_usingBlock(
                Some(ns_string!("NSApplicationDidFinishLaunchingNotification")),
                None,
                None,
                &block,
            );
        log::debug!(target: "shell", "waiting for app launch to set the Dock icon");
    }
}

/// Hands the embedded PNG to the shared `NSApplication`.
fn set() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    const ICON: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ui/assets/images/app-icon.png"
    ));

    unsafe {
        let data: *mut AnyObject = msg_send![
            class!(NSData),
            dataWithBytes: ICON.as_ptr() as *const core::ffi::c_void,
            length: ICON.len()
        ];
        if data.is_null() {
            log::warn!(target: "shell", "the embedded Dock icon could not be loaded");
            return;
        }
        let image: *mut AnyObject = msg_send![class!(NSImage), alloc];
        let image: *mut AnyObject = msg_send![image, initWithData: data];
        // `initWithData:` returns nil for a PNG it cannot decode, and passing
        // that nil straight to `setApplicationIconImage:` is what this whole file
        // exists to avoid — the `Window.icon` route silently does nothing, and so
        // does this one if the embedded bytes are wrong.
        if image.is_null() {
            log::warn!(
                target: "shell",
                "the embedded Dock icon is not a decodable image ({} bytes); the Dock will \\
                 show the default icon",
                ICON.len()
            );
            return;
        }
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, setApplicationIconImage: image];
        // `setApplicationIconImage:` retains the image; balance our alloc/init.
        let _: () = msg_send![image, release];
        log::debug!(target: "shell", "the Dock icon was set from the embedded PNG");
    }
}
