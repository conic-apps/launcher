// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

// Slint-generated code uses `unwrap()` extensively; the crate-level deny below
// stays in place for hand-written code only.
#[allow(clippy::unwrap_used)]
mod slint_backend {
    slint::include_modules!();
}

use slint_backend::App;

use log::LevelFilter;
use slint::ComponentHandle;
use slint_window::WindowService;

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;

const GAME_PAGE: &str = "game";
const SETTINGS_PAGE: &str = "settings";

/// Selects the bundled translation that best matches the system locale.
/// Catalogs live in `i18n/<language>/LC_MESSAGES/conic-launcher-slint.po`.
/// `CONIC_LOCALE` overrides the system locale (handy for testing).
fn select_locale() {
    let language =
        std::env::var("CONIC_LOCALE")
            .ok()
            .unwrap_or_else(|| match sys_locale::get_locale() {
                Some(locale) if locale.to_ascii_lowercase().starts_with("zh") => "zh_CN".into(),
                _ => "en_US".into(),
            });
    if let Err(error) = slint::select_bundled_translation(&language) {
        log::warn!("failed to select locale '{language}': {error}");
    } else {
        log::debug!(target: "app", "selected locale '{language}'");
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();

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

    // Pick the bundled translation closest to the system locale. Must run after
    // a component exists (that's what installs the translation bundle).
    select_locale();

    // Platform (mirrors crates/platform; tauri-free variant).
    let platform = slint_platform::PLATFORM_INFO.clone();
    ui.set_macos(platform.os_family == slint_platform::OsFamily::Macos);
    log::info!(
        "detected platform: {:?} ({})",
        platform.os_type,
        platform.os_family
    );

    // On macOS the window is transparent so the titlebar blend works; native
    // caption bars elsewhere must not leave a transparent client area. This is
    // handled by the `background` binding in app.slint.

    // The search placeholder is translated in app.slint via `@tr`; the hotkey
    // is platform-specific (not translated). TODO(migration): pull both from
    // config once settings are wired in.
    ui.set_search_hotkey(if ui.get_macos() { "⌘/" } else { "Ctrl+/" }.into());
    ui.set_current_page(GAME_PAGE.into());

    let window = WindowService::new(ui.clone_strong());
    log::debug!(target: "shell", "init window state — maximized: {}", window.is_maximized());

    // The custom title bar is 44px tall, but AppKit's native title bar is 32px,
    // so the traffic lights need nudging into place. This is done the Electron
    // way (see `install_traffic_light_observers`): the title-bar container is
    // resized and each button positioned explicitly, and the container is hidden
    // until the window is shown so AppKit never draws the default position.
    #[cfg(target_os = "macos")]
    install_traffic_light_observers();

    // Navigation.
    let weak = ui.as_weak();
    ui.on_open_home(move || {
        log::debug!(target: "shell", "navigate to 'game'");
        if let Some(ui) = weak.upgrade() {
            ui.set_current_page(GAME_PAGE.into());
        }
    });

    let weak = ui.as_weak();
    ui.on_open_settings(move || {
        log::debug!(target: "shell", "navigate to 'settings'");
        if let Some(ui) = weak.upgrade() {
            ui.set_current_page(SETTINGS_PAGE.into());
        }
    });

    // TODO(migration): wire these to the real stores/panels.
    ui.on_open_search(move || {
        log::info!(target: "shell", "search activated (placeholder)");
    });
    ui.on_toggle_music(move || {
        log::info!(target: "shell", "toggle music (placeholder)");
    });

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
