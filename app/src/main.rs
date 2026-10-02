// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

// Slint-generated code uses `unwrap()` extensively; the crate-level deny above
// stays in place for hand-written code only.
#[allow(clippy::unwrap_used)]
pub(crate) mod slint_backend {
    slint::include_modules!();
}

mod account_add;
mod account_avatar;
mod background;
mod command_palette;
mod config_bridge;
mod content;
mod create_instance;
mod formatting;
mod game;
mod instance_settings;
mod instance_view;
mod json;
mod launch;
mod logs;
// The Markdown/HTML renderer behind the content detail panels. It came in from
// a standalone crate and keeps that crate's habit of `unwrap`/`expect` in
// internal invariants, so the crate-level deny is opted out of here.
#[allow(clippy::unwrap_used)]
mod markdown;
mod multiplayer;
mod music;
mod native;
mod runtime;
mod scroll_input;
mod settings;
mod setup;
mod worldmap;

use std::{cell::RefCell, rc::Rc};

use slint::{ComponentHandle, Timer, Weak};

use slint_backend::{App, AppConfig};
use window::WindowService;

/// Claims the single-instance role, waiting out a relaunch.
///
/// A normal launch tries once. A process started by [`config_bridge::relaunch`]
/// carries `CONIC_RELAUNCH`, and the parent it is replacing is still releasing
/// the claim, so it retries for a few seconds before giving up.
fn acquire_single_instance() -> Option<single_instance::SingleInstance> {
    let relaunch = std::env::var_os("CONIC_RELAUNCH").is_some();
    let attempts = if relaunch { 50 } else { 1 };
    for attempt in 0..attempts {
        match single_instance::try_acquire() {
            Ok(instance) => return Some(instance),
            Err(_) if attempt + 1 < attempts => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
    None
}

fn main() {
    // Create the data directory layout before anything reads from it — the
    // logger writes into it.
    storage::LOCATIONS.init();
    logs::init();

    // Claim the single-instance role before anything else: a second launch of
    // the app is not a second window, it is this window coming forward. The
    // claim has to happen before the window exists, and the launches that come
    // with it arrive long after this function has moved on — the running
    // instance cannot be told about them yet, so they queue up until the
    // watcher below picks them up.
    let Some(single_instance) = acquire_single_instance() else {
        // The instance that is already running has just been told about this
        // launch, so this process has nothing left to do but go away.
        log::info!(target: "shell", "another instance is already running");
        return;
    };

    // The platform's winit backend hook — the macOS transparent titlebar, the
    // Windows frameless window — has to be installed before any window exists,
    // which is why it runs here rather than beside `native::install` below.
    native::install_backend();

    let ui = App::new().expect("failed to construct the app UI");

    // Configuration.
    let config = config::load_config_file().unwrap_or_else(|error| {
        log::error!("failed to load config: {error}");
        config::Config::default()
    });
    // Tell the HTTP client whether to go through the system proxy, like
    // `crates/config` does for `shared::HTTP_CLIENT`. Has to happen before the
    // first request, which is why it is done here rather than when a version
    // list is first fetched. `shared` owns the one client the whole app shares,
    // so one call reaches every crate that uses it.
    shared::set_system_proxy(config.download.use_system_proxy);

    // Pick the bundled translation. Must run after a component exists (that's
    // what installs the translation bundle).
    config_bridge::select_locale(config.language.as_deref());

    // Platform.
    let platform = platform::PLATFORM_INFO.clone();
    ui.set_macos(platform.os_family == platform::OsFamily::Macos);
    ui.set_linux(platform.os_family == platform::OsFamily::Linux);
    ui.set_windows(platform.os_family == platform::OsFamily::Windows);
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

    // The window background comes first: the game view reports the current
    // instance to it as it is set up, and that report has to land somewhere.
    background::controller::setup(&ui, Rc::clone(&shared));

    // Settings + game view + overlay "scripts".
    settings::wire(&ui, Rc::clone(&shared), Rc::clone(&save_timer));
    game::setup(&ui, Rc::clone(&shared));
    instance_settings::setup(&ui, Rc::clone(&shared));
    content::setup(&ui);
    launch::setup(&ui, Rc::clone(&shared));
    create_instance::setup(&ui, Rc::clone(&shared));
    account_add::setup(&ui);
    // The first-run wizard: the import-instances screen's two "create a blank
    // instance" buttons, the platform answer its Java screen asks for, and the
    // storage step that commits the location choice when the wizard finishes.
    setup::setup(&ui, Rc::clone(&shared));
    multiplayer::setup(&ui);
    // The clock and the wheel/trackpad classification the scroll containers use.
    scroll_input::setup(&ui);
    // The saves panel's world map. It reads the clock above for its tile fades,
    // and its own component reports the world and the visible range, so it is
    // wired after both.
    worldmap::setup(&ui);
    // The background-music player: it runs for the whole session rather than
    // per page.
    music::setup(&ui);
    // The command palette, mounted on the same layer — it opens from the title
    // bar's search field and from the `Ctrl`/`⌘` + `/` shortcut, so it is up for
    // the whole session too. Its two openers are wired in the view, the way the
    // title bar's other actions are: `CommandPaletteState.open()` for the search
    // field's click, and `CommandPaletteState.toggle()` for the `Ctrl`/`⌘` + `/`
    // shortcut, which the app-level key scope in `app.slint` binds.
    command_palette::setup(&ui);

    let window = WindowService::new(ui.clone_strong());

    // Everything the platform has to do differently: the AppKit traffic lights
    // and Dock icon, the Windows caption buttons and the frame they sit in.
    native::install(&ui);

    let minimize_window = window.clone();
    ui.on_minimize_window(move || {
        minimize_window.minimize();
    });

    // Settings → Data storage changed a root: start a fresh process and stop
    // this one. See `config_bridge::relaunch` for the hand-over.
    ui.on_restart_app(config_bridge::relaunch);

    // The window system's own close — the macOS traffic light, `Alt`+`F4`, a
    // window menu's Close, a taskbar's close — asks here first.
    //
    // This is where the platform's own close arrives. The title bar's
    // `close-window` callback covers the controls *this app draws* (Linux) and
    // the caption buttons Windows substitutes (`native/windows/caption.rs` sends
    // `SC_CLOSE`, which does come through Slint) — but on macOS the red button is
    // AppKit's, and it closes the window without the UI ever seeing it. Asking at
    // the window covers all three, and is also where a close that is not a button
    // at all belongs.
    //
    // `Dialogs.confirm-quit-app-visible` is the flag the title bar's callback sets
    // too, which is what makes it the "already answered" test below as well as
    // the dialog's own: `App.close()` at the end of the exit animation comes back
    // through here, and the only close that must not be re-asked about is that one.
    window.window().on_close_requested({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else {
                return slint::CloseRequestResponse::HideWindow;
            };
            let dialogs = ui.global::<slint_backend::Dialogs>();
            // Already on the way out: this is `App`'s own `close()` after the exit
            // animation, and answering anything but `HideWindow` here would
            // cancel the quit the user just confirmed.
            if dialogs.get_confirm_quit_app_visible() {
                return slint::CloseRequestResponse::HideWindow;
            }
            if ui.global::<slint_backend::Navigation>().get_current_page() == "launch" {
                dialogs.set_confirm_quit_app_visible(true);
                return slint::CloseRequestResponse::KeepWindowShown;
            }
            slint::CloseRequestResponse::HideWindow
        }
    });

    // The music player's background volume follows the window's focus (the
    // store's `onFocusChanged`). Registered here, where the window service exists.
    music::watch_focus(&window);

    // Drag regions (`globals/window-drag.slint`): a press on one — the dialog's
    // scrim — moves the window.
    ui.global::<slint_backend::WindowDrag>().on_start({
        let window = window.clone();
        move || {
            log::debug!(target: "shell", "window drag requested");
            window.drag_window();
        }
    });
    log::debug!(target: "shell", "init window state — maximized: {}", window.is_maximized());

    // The app is the only instance of it now, so it can be told about the next
    // launch. The claim moves onto the watcher's thread, which is where the
    // launches arrive, and stays there for as long as the process lives.
    //
    // A weak handle is what crosses over: a strong one cannot be moved onto the
    // watcher's thread, but `Weak` is `Send` (and `Sync`).
    watch_launches(single_instance, ui.as_weak());

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

    // A fresh install lands on the setup wizard rather than the game view. The
    // flag is stored in the config, so a wizard that was finished or skipped
    // does not come back; an older config without the key reads it as `false`
    // and is shown the wizard once.
    //
    // Set before `run` so the wizard is the first frame, and after the config is
    // loaded and applied, since the wizard's steps read the `AppConfig` global.
    if !shared.borrow().setup_completed {
        ui.global::<slint_backend::Navigation>()
            .set_current_page("setup".into());
    }

    ui.run().expect("failed to run the shell event loop");

    // On exit the multiplayer poll thread is joined and the Conic Nexus session
    // destroyed.
    multiplayer::shutdown();

    cleanup_temp_folder();
}

/// Removes the per-run scratch directory [`storage::LOCATIONS`] creates.
///
/// The installers stage a bootstrapper jar here (`install`), and the
/// directory is `create_dir_all`-ed in a fresh UUID-named path on every launch,
/// so without this it accumulates one directory per run in the OS temp folder.
fn cleanup_temp_folder() {
    match std::fs::remove_dir_all(&storage::LOCATIONS.launcher.temp) {
        Ok(_) => log::info!("Temporary files cleared"),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            log::error!("Could not clear temp folder: {error}")
        }
        _ => (),
    }
}

/// Brings the window forward for every later launch of the app.
///
/// Each later launch brings the running window forward through
/// [`WindowService::bring_to_front`].
///
/// The launches arrive on a platform thread — a D-Bus worker, a `WM_COPYDATA`
/// window message, a socket reader — so the window is only ever touched from
/// the event loop, which is what the weak handle is upgraded in. `SingleInstance`
/// moves in here as well: it holds the single-instance claim, which has to
/// outlive the setup in `main`.
fn watch_launches(single_instance: single_instance::SingleInstance, app: Weak<App>) {
    std::thread::Builder::new()
        .name("conic-single-instance".into())
        .spawn(move || {
            while let Some(launch) = single_instance.next_launch() {
                log::info!(
                    target: "shell",
                    "another launch was handed over: {:?} (in {})",
                    launch.args,
                    launch.cwd
                );
                // TODO: the arguments are the deep-link payload the accounts
                // view consumes (the Microsoft login's `?code=…`, reached
                // through the desktop entry's `conic-launcher://` handler) —
                // route them there.
                let app = app.clone();
                if let Err(error) = app.upgrade_in_event_loop(move |app| {
                    WindowService::new(app).bring_to_front();
                }) {
                    log::debug!(target: "shell", "the window was not brought forward: {error}");
                }
            }
        })
        .expect("failed to start the single-instance watcher");
}
