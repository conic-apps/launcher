// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Sets the macOS Dock and app-switcher icon, which the Slint `Window.icon`
//! binding cannot reach; Windows and Linux use it in `app.slint`.

/// Defers [`set`] until the app has finished launching: AppKit discards an icon
/// set earlier during its own launch.
pub(super) fn install() {
    use core::ptr::NonNull;
    use objc2_foundation::{NSNotification, NSNotificationCenter, ns_string};

    let block = block2::RcBlock::new(move |_notification: NonNull<NSNotification>| {
        set();
    });

    unsafe {
        // The observer token is dropped on purpose: the centre keeps the block
        // alive for the process.
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

    const ICON: &[u8] = conic_ui::assets::APP_ICON_PNG;

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
        // `initWithData:` returns nil for a PNG it cannot decode, and
        // `setApplicationIconImage:` silently does nothing with nil.
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
