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

mod support;
mod ui;
mod usecases;

use std::{cell::RefCell, rc::Rc};

use log::info;
use slint::{ComponentHandle, Timer, Weak};

use slint_backend::{App, AppConfig};
use window::WindowService;

/// Claims the single-instance role, waiting out a relaunch.
///
/// A normal launch tries once. A process started by [`ui::services::app_config::relaunch`]
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
    support::logs::init();

    info!("Conic Launcher is starting up");
    info!(
        "Conic Launcher is open source, You can view the source code on Github: https://github.com/conic-apps/launcher"
    );

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
    // which is why it runs here rather than beside `support::native::install` below.
    support::native::install_backend();

    let ui = App::new().expect("failed to construct the app UI");

    // The version this build was made as, for the User-Agent, the JVM's
    // `launcher_version` and the self-update check. `shared` cannot read it
    // itself — its manifest version is the placeholder `0.0.0` — so the app
    // hands its compiled-in version over. It must land before the config load:
    // `UpdateChannel`'s default derives from it, seeding `update_channel =
    // "beta"` for a pre-release build rather than `stable`.
    shared::set_app_version(env!("CARGO_PKG_VERSION"));

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

    // Same before-the-first-request shape as the proxy preference: the cache is
    // settled before anything is fetched through it. It is a directory rather
    // than a setting because it is not a preference — there is only one place it
    // could sensibly go.
    shared::http_cache::set_dir(storage::LOCATIONS.launcher.cache.join("http"));

    // Pick the bundled translation. Must run after a component exists (that's
    // what installs the translation bundle).
    ui::services::app_config::select_locale(config.language.as_deref());

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
    ui::services::app_config::apply_config(&settings, &config);

    let shared = Rc::new(RefCell::new(config));
    let save_timer = Rc::new(Timer::default());

    // The window background comes first: the game view reports the current
    // instance to it as it is set up, and that report has to land somewhere.
    ui::components::background::controller::setup(&ui, Rc::clone(&shared));

    // The avatar's corner mask. It has to be in place before any account is
    // drawn, so it is wired with the other components rather than a view.
    ui::components::account_avatar::setup(&ui);

    ui::views::settings::wire(&ui, Rc::clone(&shared), Rc::clone(&save_timer));
    ui::views::game::setup(&ui, Rc::clone(&shared));
    ui::overlays::account_view::setup(&ui, Rc::clone(&shared));
    ui::overlays::instance_settings::setup(&ui, Rc::clone(&shared));
    ui::overlays::content::setup(&ui);
    ui::views::launch::setup(&ui, Rc::clone(&shared));
    // The crash page: it subscribes to the launch crate's session events for
    // the whole app, so it has to be wired before a game can ever be launched.
    ui::views::crash::setup(&ui);
    ui::overlays::dialogs::create_instance::setup(&ui, Rc::clone(&shared));
    ui::overlays::dialogs::account_add::setup(&ui);
    ui::views::setup::setup(&ui, Rc::clone(&shared));
    ui::overlays::dialogs::multiplayer::setup(&ui);
    // The clock and the wheel/trackpad classification the scroll containers use.
    ui::services::scroll::setup(&ui);
    // The generic tooltip's pointer position: Slint has no global cursor
    // accessor, so the window's own `CursorMoved` events feed `TooltipState`.
    ui::services::tooltip::setup(&ui);
    // The saves panel's world map. It reads the clock above for its tile fades,
    // and its own component reports the world and the visible range, so it is
    // wired after both.
    ui::overlays::content::world_map::setup(&ui);
    // The background-music player: it runs for the whole session rather than
    // per page.
    ui::overlays::music_player::setup(&ui);
    // The news browser: the title bar's newspaper button opens it, and it
    // fetches the feeds itself the first time it does.
    ui::overlays::news::setup(&ui);
    // The command palette, on the same whole-session layer as the news browser;
    // it opens from the title bar's search field and the `Ctrl`/`⌘` + `/`
    // shortcut, both bound in the view.
    ui::overlays::command_palette::setup(&ui);

    // Self-update, run silently. The install form (AppImage / .app / MSI /
    // portable / package manager) is detected once and decides whether the
    // settings rows are live; a package-manager install is never checked.
    // Everything else checks in the background and only *stages* — the swap
    // happens after `ui.run` returns, and every step goes to the log.
    {
        ui.global::<AppConfig>()
            .set_update_self_updating(usecases::update::is_self_updating());
        if shared.borrow().auto_update && usecases::update::is_self_updating() {
            spawn_update(&shared);
        }
    }

    let window = WindowService::new(ui.clone_strong());

    // Everything the platform has to do differently: the AppKit traffic lights
    // and Dock icon, the Windows caption buttons and the frame they sit in.
    support::native::install(&ui);

    let minimize_window = window.clone();
    ui.on_minimize_window(move || {
        minimize_window.minimize();
    });

    // Settings → Data storage changed a root: start a fresh process and stop
    // this one. See `ui::services::app_config::relaunch` for the hand-over.
    ui.on_restart_app(ui::services::app_config::relaunch);

    // Every close asks here first: the title bar's controls, the Windows caption
    // buttons (their `SC_CLOSE` does come through Slint), macOS's AppKit red
    // button (which closes without the UI ever seeing it), and `⌘W`/`⌘Q` — both
    // call `App.close()`. Asking at the window is what covers all of them.
    //
    // `Dialogs.confirm-quit-app-visible` is the "already answered" test as well
    // as the dialog's own flag: `App.close()` at the end of the exit animation
    // comes back through here, and that close must not be re-asked.
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
    ui::overlays::music_player::watch_focus(&window);

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

    // A fresh install lands on the setup wizard, not the game view; the flag is
    // stored, so finishing or skipping it is final, and a config without the key
    // reads `false` and shows it once. Set before `run` (first frame) and after
    // the config is applied, since the wizard's steps read `AppConfig`.
    if !shared.borrow().setup_completed {
        ui.global::<slint_backend::Navigation>()
            .set_current_page("setup".into());
    }

    // The platform's terminate requests — macOS `⌘Q`, the menu's Quit, a system
    // logout — must run the same close flow as `⌘W`, whose launch-page confirm
    // lives in the window's close-requested callback. `request-close` replays
    // them as that close; it hops through the event loop because AppKit asks
    // outside Slint's dispatch. `run_exit_work` is the fallback for a terminate
    // that really ends the process.
    let request_close = {
        let weak = ui.as_weak();
        move || {
            let _ = weak.upgrade_in_event_loop(|ui| {
                ui.invoke_request_close();
            });
        }
    };
    support::native::install_terminate_handler(request_close, run_exit_work);

    ui.run().expect("failed to run the shell event loop");

    run_exit_work();
}

/// The work that has to happen as the process goes away, whichever way it was
/// asked to.
///
/// It runs once: a window close returns from `ui.run` and calls it here, and
/// [`support::native::install_terminate_handler`] routes a terminate that really
/// ends the process through it too. The exit-time update apply must not run
/// twice, so a second caller finds the gate already shut.
fn run_exit_work() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }

    // The updater swaps in the staged bundle once the event loop has returned:
    // the window is gone and nothing will read the running executable again.
    match update::apply_pending() {
        Ok(true) => log::info!("a staged update was applied; it takes effect next launch"),
        Ok(false) => {}
        Err(error) => log::error!("failed to apply the staged update: {error}"),
    }

    // On exit the multiplayer poll thread is joined and the Conic Nexus session
    // destroyed.
    ui::overlays::dialogs::multiplayer::shutdown();

    // The clipboard handle has to be dropped before the process exits
    // (`arboard` keeps a background thread that serves the selection); the
    // event loop has already returned, so this is the drop point.
    ui::services::app_config::drop_clipboard();

    cleanup_temp_folder();
}

/// Starts one background update check + staging download.
///
/// It is silent: the use case logs what it does, and the staged bundle is
/// applied as the process exits. Nothing here touches the interface.
fn spawn_update(shared: &Rc<RefCell<config::Config>>) {
    let channel = shared.borrow().update_channel.clone();
    crate::support::runtime::spawn(usecases::update::run(channel));
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
/// The launches arrive on a platform thread — a D-Bus worker, a `WM_COPYDATA`
/// window message, a socket reader — so the window is only ever touched from the
/// event loop, which is what the weak handle is upgraded in. `SingleInstance`
/// moves in here because it holds the claim, which has to outlive `main`.
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
