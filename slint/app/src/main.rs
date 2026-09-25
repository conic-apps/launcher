// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

// Slint-generated code uses `unwrap()` extensively; the crate-level deny below
// stays in place for hand-written code only.
#[allow(clippy::unwrap_used)]
pub(crate) mod slint_backend {
    slint::include_modules!();
}

mod config_bridge;
mod game;
mod java;
mod runtime;
mod settings;

use std::{cell::RefCell, rc::Rc};

use log::LevelFilter;
use slint::{ComponentHandle, Timer};

use slint_backend::{App, AppConfig};
use slint_window::WindowService;

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();

    // Create the data directory layout (shares `~/.conic[-debug]` with the
    // Tauri app) before anything reads from it.
    slint_folder::DATA_LOCATION.init();

    // macOS gets the Chrome-style window: a native titled window with a
    // transparent, title-less titlebar and a full-size content view, so the
    // custom title bar draws under the real AppKit traffic lights while all
    // native behavior (drag, resize, corner rounding, fullscreen) keeps
    // working. The hook runs before every winit window is created.
    #[cfg(target_os = "macos")]
    {
        use slint::platform::set_platform;
        use winit::platform::macos::WindowAttributesExtMacOS;

        let backend = i_slint_backend_winit::Backend::builder()
            .with_window_attributes_hook(|attributes| {
                attributes
                    .with_titlebar_transparent(true)
                    .with_title_hidden(true)
                    .with_fullsize_content_view(true)
            })
            .build()
            .expect("failed to build the winit backend");
        set_platform(Box::new(backend)).expect("failed to install the winit backend");
    }

    let ui = App::new().expect("failed to construct the app UI");

    // Configuration (shared with the Tauri app).
    let config = slint_config::load_config_file().unwrap_or_else(|error| {
        log::error!("failed to load config: {error}");
        slint_config::Config::default()
    });
    // Tell the HTTP client whether to go through the system proxy, like
    // `crates/config` does for `shared::HTTP_CLIENT`. Has to happen before the
    // first request, which is why it is done here rather than when a version
    // list is first fetched.
    slint_install::set_system_proxy(config.download.use_system_proxy);

    // Pick the bundled translation. Must run after a component exists (that's
    // what installs the translation bundle).
    config_bridge::select_locale(config.language.as_deref());

    // Platform (mirrors crates/platform; tauri-free variant).
    let platform = slint_platform::PLATFORM_INFO.clone();
    ui.set_macos(platform.os_family == slint_platform::OsFamily::Macos);
    log::info!(
        "detected platform: {:?} ({})",
        platform.os_type,
        platform.os_family
    );

    // The search placeholder is translated in app.slint via `@tr`; the hotkey
    // is platform-specific (not translated).
    ui.set_search_hotkey(if ui.get_macos() { "⌘/" } else { "Ctrl+/" }.into());

    // Seed the settings global and keep the in-memory config in sync with it.
    let settings = ui.global::<AppConfig>();
    config_bridge::apply_config(&settings, &config);

    let shared = Rc::new(RefCell::new(config));
    let save_timer = Rc::new(Timer::default());

    // Settings + game view "scripts".
    settings::wire(&ui, Rc::clone(&shared), Rc::clone(&save_timer));
    game::setup(&ui, Rc::clone(&shared));

    // TODO(migration): wire these to the real command palette / music player.
    ui.on_open_search(move || {
        log::info!(target: "shell", "search activated (placeholder)");
    });
    ui.on_toggle_music(move || {
        log::info!(target: "shell", "toggle music (placeholder)");
    });

    // The custom title bar is 44px tall, but AppKit's native title bar is 32px,
    // so the traffic lights need nudging into place. This is done the Electron
    // way (see `install_traffic_light_observers`): the title-bar container is
    // resized and each button positioned explicitly, and the container is hidden
    // until the window is shown so AppKit never draws the default position.
    #[cfg(target_os = "macos")]
    install_traffic_light_observers();

    // macOS draws the Dock icon from NSApplication, not from the window, and the
    // icon can only be set once `applicationDidFinishLaunching` has run (inside
    // `ui.run()`) — setting it earlier is overwritten by AppKit during launch.
    #[cfg(target_os = "macos")]
    install_app_icon_observer();

    let window = WindowService::new(ui.clone_strong());
    log::debug!(target: "shell", "init window state — maximized: {}", window.is_maximized());

    // Double-clicking the title bar zooms (macOS: native fullscreen space,
    // elsewhere: maximized), matching the system convention.
    let macos = ui.get_macos();
    let ws = window.clone();
    ui.on_maximize_or_fullscreen(move || {
        if macos {
            log::debug!(target: "shell", "toggle fullscreen");
            ws.toggle_fullscreen();
        } else {
            log::debug!(target: "shell", "toggle maximized");
            ws.toggle_maximize();
        }
    });

    ui.run().expect("failed to run the shell event loop");
}

/// Sets the macOS Dock / task-switcher icon.
///
/// The Slint `Window.icon` binding can't reach it: the winit backend forwards
/// that to `winit::Window::set_window_icon`, which is a no-op on macOS. Instead
/// the embedded PNG is loaded into an `NSImage` and handed to the shared
/// `NSApplication`. Windows and Linux use the `icon:` binding in `app.slint`.
#[cfg(target_os = "macos")]
fn set_macos_app_icon() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    const ICON: &[u8] = include_bytes!("../ui/assets/app-icon.png");

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

/// Defers `set_macos_app_icon` until the app has finished launching.
///
/// `setApplicationIconImage:` only sticks after `applicationDidFinishLaunching:`
/// (which AppKit runs inside `ui.run()`); setting it before that is discarded
/// when AppKit initializes the app icon during launch.
#[cfg(target_os = "macos")]
fn install_app_icon_observer() {
    use core::ptr::NonNull;
    use objc2_foundation::{NSNotification, NSNotificationCenter, ns_string};

    let block = block2::RcBlock::new(move |_notification: NonNull<NSNotification>| {
        set_macos_app_icon();
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

/// Installs the observers that keep the traffic lights in place, mirroring
/// Electron's `WindowButtonsProxy` + `ElectronNSWindowDelegate`:
///
/// * the title-bar container is hidden from the pre-show update until the window
///   is visible, so AppKit never draws the buttons at their default position
///   during the show;
/// * the geometry is re-applied on window resize and on become/resign key/main
///   (Electron's exact set of triggers) — *not* on every `DidUpdate`.
#[cfg(target_os = "macos")]
fn install_traffic_light_observers() {
    use core::ptr::NonNull;
    use objc2_foundation::{NSNotification, NSNotificationCenter, ns_string};
    use std::cell::Cell;
    use std::rc::Rc;

    let revealed = Rc::new(Cell::new(false));

    // Visibility transitions: hide the container while the window is not on
    // screen (AppKit would otherwise draw the default position), reveal it and
    // position the buttons once it is. Re-applying on every update keeps the
    // geometry correct after AppKit's own re-layouts (e.g. when revealing).
    let revealed_for_visibility = revealed.clone();
    let visibility_block = block2::RcBlock::new(move |notification: NonNull<NSNotification>| {
        let notification = unsafe { notification.as_ref() };
        let Some(window) = (unsafe { notification.object() }) else {
            return;
        };
        let window = &*window as *const AnyObject as *mut AnyObject;
        if unsafe { window_visible(window) } {
            if !revealed_for_visibility.get() {
                revealed_for_visibility.set(true);
                set_titlebar_container_hidden(window, false);
            }
            reposition_ns_window(window);
        } else {
            set_titlebar_container_hidden(window, true);
        }
    });

    // Discrete relayout events (Electron re-applies on exactly these).
    let revealed_for_relayout = revealed.clone();
    let relayout_block = block2::RcBlock::new(move |notification: NonNull<NSNotification>| {
        let notification = unsafe { notification.as_ref() };
        let Some(window) = (unsafe { notification.object() }) else {
            return;
        };
        let window = &*window as *const AnyObject as *mut AnyObject;
        reposition_ns_window(window);
        if !revealed_for_relayout.get() {
            set_titlebar_container_hidden(window, true);
        }
    });

    unsafe {
        let center = NSNotificationCenter::defaultCenter();
        center.addObserverForName_object_queue_usingBlock(
            Some(ns_string!("NSWindowDidUpdateNotification")),
            None,
            None,
            &visibility_block,
        );
        for name in [
            ns_string!("NSWindowDidResizeNotification"),
            ns_string!("NSWindowDidBecomeKeyNotification"),
            ns_string!("NSWindowDidResignKeyNotification"),
            ns_string!("NSWindowDidBecomeMainNotification"),
            ns_string!("NSWindowDidResignMainNotification"),
        ] {
            center.addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                None,
                &relayout_block,
            );
        }
    }
}

/// Whether the given `NSWindow` is currently on screen.
#[cfg(target_os = "macos")]
unsafe fn window_visible(ns_window: *mut AnyObject) -> bool {
    use objc2::msg_send;

    let visible: bool = unsafe { msg_send![ns_window, isVisible] };
    visible
}

/// Hides/shows the whole title-bar container (`WindowButtonsProxy.setVisible:`).
#[cfg(target_os = "macos")]
fn set_titlebar_container_hidden(ns_window: *mut AnyObject, hidden: bool) {
    use objc2::msg_send;
    use objc2::runtime::Bool;

    let Some(container) = (unsafe { titlebar_container(ns_window) }) else {
        return;
    };
    unsafe {
        let current: bool = msg_send![container, isHidden];
        if current != hidden {
            let value = if hidden { Bool::YES } else { Bool::NO };
            let _: () = msg_send![container, setHidden: value];
        }
    }
}

/// Returns the `NSTitlebarContainerView` (the buttons' superview's superview).
#[cfg(target_os = "macos")]
unsafe fn titlebar_container(ns_window: *mut AnyObject) -> Option<*mut AnyObject> {
    use objc2::msg_send;

    unsafe {
        let close: *mut AnyObject = msg_send![ns_window, standardWindowButton: 0u64];
        if close.is_null() {
            return None;
        }
        let titlebar: *mut AnyObject = msg_send![close, superview];
        if titlebar.is_null() {
            return None;
        }
        let container: *mut AnyObject = msg_send![titlebar, superview];
        (!container.is_null()).then_some(container)
    }
}

/// Moves the native AppKit traffic lights (close/minimize/zoom) to match the
/// custom 44px title bar: left edges at 16/38/60 and their top edge 15px below
/// the window top (centre 22). The `NSTitlebarContainerView` is resized to 38px
/// (AppKit then places the buttons and keeps their tracking areas right); the
/// buttons themselves are only nudged horizontally.
#[cfg(target_os = "macos")]
fn reposition_ns_window(ns_window: *mut AnyObject) {
    use objc2::msg_send;
    use objc2_foundation::{CGPoint, CGRect};

    /// Left edge of each button.
    const BUTTON_X: [f64; 3] = [16.0, 38.0, 60.0];
    /// AppKit keeps the buttons 9px above the container's bottom, so a 38px
    /// container puts their top 15px below the window top.
    const TITLE_BAR_HEIGHT: f64 = 38.0;
    /// NSWindowCloseButton = 0, NSWindowMiniaturizeButton = 1, NSWindowZoomButton = 2.
    const BUTTONS: [u64; 3] = [0, 1, 2];

    if ns_window.is_null() {
        return;
    }
    unsafe {
        // Skip while fullscreen: the title bar is hidden there.
        let style: usize = msg_send![ns_window, styleMask];
        if style & (1 << 14) != 0 {
            return;
        }
        let Some(container) = titlebar_container(ns_window) else {
            return;
        };

        let window_frame: CGRect = msg_send![ns_window, frame];
        let mut container_frame: CGRect = msg_send![container, frame];
        let target_y = window_frame.size.height - TITLE_BAR_HEIGHT;
        if container_frame.size.height != TITLE_BAR_HEIGHT || container_frame.origin.y != target_y {
            container_frame.size.height = TITLE_BAR_HEIGHT;
            container_frame.origin.y = target_y;
            let _: () = msg_send![container, setFrame: container_frame];
        }

        for (kind, x) in BUTTONS.into_iter().zip(BUTTON_X) {
            let button: *mut AnyObject = msg_send![ns_window, standardWindowButton: kind];
            if button.is_null() {
                continue;
            }
            let frame: CGRect = msg_send![button, frame];
            if frame.origin.x != x {
                let _: () = msg_send![button, setFrameOrigin: CGPoint::new(x, frame.origin.y)];
            }
        }
    }
}
