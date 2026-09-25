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

mod cjk_font;
mod config_bridge;
mod create_instance;
mod game;
mod java;
mod runtime;
mod scroll_input;
mod settings;

#[cfg(target_os = "macos")]
mod traffic_lights;

use std::{cell::RefCell, rc::Rc};

use log::LevelFilter;
use slint::{ComponentHandle, Timer};

use slint_backend::{App, AppConfig};
use slint_window::WindowService;

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

    // The custom title bar is 44px tall, so the native traffic lights have to
    // sit lower than AppKit's own title bar would put them. This has to happen
    // before the first window exists — see the `traffic_lights` module.
    #[cfg(target_os = "macos")]
    traffic_lights::install();

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

    // Settings + game view + overlay "scripts".
    settings::wire(&ui, Rc::clone(&shared), Rc::clone(&save_timer));
    game::setup(&ui, Rc::clone(&shared));
    create_instance::setup(&ui, Rc::clone(&shared));
    // The clock and the wheel/trackpad classification the scroll containers use.
    scroll_input::setup(&ui);

    // TODO(migration): wire these to the real command palette / music player.
    ui.on_open_search(move || {
        log::info!(target: "shell", "search activated (placeholder)");
    });
    ui.on_toggle_music(move || {
        log::info!(target: "shell", "toggle music (placeholder)");
    });

    // The title bar stops leaving room for the traffic lights while they are
    // hidden by fullscreen. Registered after `App::new()` because it reports
    // into the UI; the observer itself is armed from then on.
    #[cfg(target_os = "macos")]
    traffic_lights::watch_fullscreen(&ui);

    // macOS draws the Dock icon from NSApplication, not from the window, and the
    // icon can only be set once `applicationDidFinishLaunching` has run (inside
    // `ui.run()`) — setting it earlier is overwritten by AppKit during launch.
    #[cfg(target_os = "macos")]
    install_app_icon_observer();

    let window = WindowService::new(ui.clone_strong());

    // Drag regions (`globals/window-drag.slint`): a press on one — the dialog's
    // scrim, as in the Vue's `data-tauri-drag-region` — moves the window.
    ui.global::<slint_backend::WindowDrag>().on_start({
        let window = window.clone();
        move || {
            log::debug!(target: "shell", "window drag requested");
            window.drag_window();
        }
    });
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
