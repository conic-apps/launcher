// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

// The Slint bindings are generated in `conic-ui`; re-export them under the
// name the rest of the app already uses.
pub(crate) mod slint_backend {
    pub use conic_ui::*;
}

mod support;
mod ui;
mod usecases;

use std::{cell::RefCell, rc::Rc};

use log::info;
use slint::{ComponentHandle, Timer, Weak};

use slint_backend::{App, AppConfig, Navigation, NewsState, NotificationState};
use window::WindowService;

/// Claims the single-instance role. A relaunch retries briefly because the
/// parent it replaces is still releasing the claim.
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

    // The claim must happen before a window exists; later launches queue until
    // the watcher below picks them up.
    let Some(single_instance) = acquire_single_instance() else {
        log::info!(target: "shell", "another instance is already running");
        return;
    };

    // The winit backend hook must be installed before any window exists.
    support::native::install_backend();

    let ui = App::new().expect("failed to construct the app UI");

    // Must land before the config load: `UpdateChannel`'s default derives from it.
    shared::set_app_version(env!("CARGO_PKG_VERSION"));

    let config = config::load_config_file().unwrap_or_else(|error| {
        log::error!("failed to load config: {error}");
        config::Config::default()
    });
    // Before the first request.
    shared::set_system_proxy(config.download.use_system_proxy);
    shared::http_cache::set_dir(storage::LOCATIONS.launcher.cache.join("http"));

    // Must run after a component exists, which is what installs the translation
    // bundle.
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

    ui.set_search_hotkey(if ui.get_macos() { "⌘/" } else { "Ctrl+/" }.into());

    let settings = ui.global::<AppConfig>();
    ui::services::app_config::apply_config(&settings, &config);

    let shared = Rc::new(RefCell::new(config));
    let save_timer = Rc::new(Timer::default());

    // Before the game view, which reports the current instance to it.
    ui::components::background::controller::setup(&ui, Rc::clone(&shared));

    // Before any account is drawn.
    ui::components::account_avatar::setup(&ui);

    ui::views::settings::wire(&ui, Rc::clone(&shared), Rc::clone(&save_timer));
    ui::views::game::setup(&ui, Rc::clone(&shared));
    ui::overlays::account_view::setup(&ui, Rc::clone(&shared));
    ui::overlays::instance_settings::setup(&ui, Rc::clone(&shared));
    ui::overlays::content::setup(&ui);
    ui::views::launch::setup(&ui, Rc::clone(&shared));
    // Subscribes to the launch crate's session events, so before any launch.
    ui::overlays::crash::setup(&ui);
    ui::overlays::dialogs::create_instance::setup(&ui, Rc::clone(&shared));
    ui::overlays::dialogs::account_add::setup(&ui);
    ui::views::setup::setup(&ui, Rc::clone(&shared));
    ui::overlays::dialogs::multiplayer::setup(&ui);
    ui::services::scroll::setup(&ui);
    // Slint has no global cursor accessor; `CursorMoved` feeds `TooltipState`.
    ui::services::tooltip::setup(&ui);
    // After the clock above, whose tick drives its tile fades.
    ui::overlays::content::world_map::setup(&ui);
    ui::overlays::music_player::setup(&ui);
    ui::overlays::news::setup(&ui);
    let notices = ui::services::notifications::setup(&ui);
    // The previous session staged an update; this is the new build's first
    // chance to announce it.
    ui::services::update::notify_applied(notices.clone());
    ui::services::minecraft_update::setup(Rc::clone(&shared), notices.clone());
    ui::services::tasks::setup(&ui, notices.clone());
    ui::overlays::dialogs::modpack_import::setup(&ui, Rc::clone(&shared), notices.clone());
    {
        let weak_ui = ui.as_weak();
        let shared = Rc::clone(&shared);
        let mut previous_page = ui.global::<Navigation>().get_current_page();
        ui.global::<Navigation>().on_page_changed(move |page| {
            let left_setup = previous_page.as_str() == "setup" && page.as_str() != "setup";
            previous_page = page.clone();
            if let Some(ui) = weak_ui.upgrade() {
                if left_setup {
                    ui::views::setup::notify_if_abandoned(&ui, &shared);
                }
                ui.global::<NewsState>().set_visible(false);
                ui.global::<NotificationState>().set_visible(false);
                // `Navigation.page-changed` holds one handler, so every surface
                // that wants it is called from here; a second `on_page_changed`
                // would be replaced by this one without a word.
                ui::views::launch::page_left(&ui, page.as_str());
            }
        });
    }
    ui::overlays::command_palette::setup(&ui);
    // After every transient surface has registered its own close callback.
    ui::services::escape::setup(&ui);

    // Everything except a package-manager install checks in the background and
    // only stages; the swap happens after `ui.run` returns.
    {
        ui.global::<AppConfig>()
            .set_update_self_updating(usecases::update::is_self_updating());
        if shared.borrow().auto_update && usecases::update::is_self_updating() {
            spawn_update(&shared, notices);
        }
    }

    let window = WindowService::new(ui.clone_strong());

    // The AppKit traffic lights and Dock icon, or the Windows caption buttons.
    support::native::install(&ui);

    let minimize_window = window.clone();
    ui.on_minimize_window(move || {
        minimize_window.minimize();
    });

    ui.on_restart_app(ui::services::app_config::relaunch);

    // Asking at the window covers every close path: title-bar controls, the
    // Windows caption, macOS's red button, and `⌘W`/`⌘Q`.
    window.window().on_close_requested({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else {
                return slint::CloseRequestResponse::HideWindow;
            };
            let dialogs = ui.global::<slint_backend::Dialogs>();
            // The exit animation's own `close()` comes back through here; do not
            // re-ask.
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

    ui::overlays::music_player::watch_focus(&window);

    // Drag regions in `globals/window-drag.slint`, such as a dialog's scrim.
    ui.global::<slint_backend::WindowDrag>().on_start({
        let window = window.clone();
        move || {
            log::debug!(target: "shell", "window drag requested");
            window.drag_window();
        }
    });
    log::debug!(target: "shell", "init window state — maximized: {}", window.is_maximized());

    // The claim moves onto the watcher's thread, so only a weak handle crosses.
    watch_launches(single_instance, ui.as_weak());

    // Double-click the title bar: fullscreen on macOS, maximized elsewhere.
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

    // After the config is applied, since the wizard's steps read `AppConfig`.
    if !shared.borrow().setup_completed {
        ui.global::<slint_backend::Navigation>()
            .set_current_page("setup".into());
    }

    // Terminate requests must run the same close flow as `⌘W`. `request-close`
    // replays them as that close; AppKit asks outside Slint's dispatch, so it
    // hops through the event loop.
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

/// Exit work, run once whether the window closed or the process is being
/// terminated. The update apply must not run twice.
fn run_exit_work() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }

    // Once the event loop has returned, nothing will read the executable again.
    match update::apply_pending() {
        Ok(true) => log::info!("a staged update was applied; it takes effect next launch"),
        Ok(false) => {}
        Err(error) => log::error!("failed to apply the staged update: {error}"),
    }

    ui::overlays::dialogs::multiplayer::shutdown();
    // `arboard` keeps a background thread serving the selection.
    ui::services::app_config::drop_clipboard();

    cleanup_temp_folder();
}

/// Starts one background update check and staging download.
fn spawn_update(shared: &Rc<RefCell<config::Config>>, notices: shared::notification::NoticeSink) {
    let channel = shared.borrow().update_channel.clone();
    crate::support::runtime::spawn(usecases::update::run(channel, notices));
}

/// Removes the per-run scratch directory. It has a fresh UUID name each launch,
/// so otherwise one directory accumulates per run.
fn cleanup_temp_folder() {
    match std::fs::remove_dir_all(&storage::LOCATIONS.launcher.temp) {
        Ok(_) => log::info!("Temporary files cleared"),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            log::error!("Could not clear temp folder: {error}")
        }
        _ => (),
    }
}

/// Brings the window forward for every later launch. Launches arrive on a
/// platform thread; the weak handle is upgraded on the event loop.
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
