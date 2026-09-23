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

mod java;

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use log::LevelFilter;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

thread_local! {
    /// Last Java scan result. Only accessed on the UI thread so it can stay an
    /// `Rc`-free, `Send`-free `RefCell` while the scan itself runs on a worker.
    static JAVA_CACHE: RefCell<Vec<java::DetectedJava>> = const { RefCell::new(Vec::new()) };
}
use slint_backend::{App, AppConfig, JavaRuntime};
use slint_window::WindowService;

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;

/// Maps a launcher language code to the bundled gettext locale (the catalog
/// folder name). All 12 launcher languages ship a catalog.
fn bundled_locale(code: &str) -> Option<&'static str> {
    match code {
        "en_us" => Some("en_US"),
        "zh_cn" => Some("zh_CN"),
        "zh_tw" => Some("zh_TW"),
        "ja_jp" => Some("ja_JP"),
        "ko_kr" => Some("ko_KR"),
        "de_de" => Some("de_DE"),
        "fr_fr" => Some("fr_FR"),
        "es_es" => Some("es_ES"),
        "pt_br" => Some("pt_BR"),
        "ru_ru" => Some("ru_RU"),
        "tr_tr" => Some("tr_TR"),
        "pl_pl" => Some("pl_PL"),
        _ => None,
    }
}

/// Resolves a launcher language code (an empty string means "follow system")
/// to a bundled locale, falling back to English.
fn resolve_locale(language: &str) -> &'static str {
    if language.is_empty() {
        bundled_locale(slint_config::get_system_language())
    } else {
        bundled_locale(language)
    }
    .unwrap_or("en_US")
}

fn apply_locale(locale: &str) {
    if let Err(error) = slint::select_bundled_translation(locale) {
        log::warn!("failed to select locale '{locale}': {error}");
    } else {
        log::debug!(target: "app", "selected locale '{locale}'");
    }
}

/// Selects the bundled translation that best matches the preferred language
/// (from the config) or the system locale. `CONIC_LOCALE` overrides both and
/// disables runtime switching.
fn select_locale(preferred: Option<&str>) {
    if let Ok(override_locale) = std::env::var("CONIC_LOCALE") {
        apply_locale(&override_locale);
        return;
    }
    let locale = match preferred {
        Some(language) if !language.is_empty() => bundled_locale(language),
        _ => bundled_locale(slint_config::get_system_language()),
    }
    .unwrap_or("en_US");
    apply_locale(locale);
}

/// Seeds the Slint `AppConfig` global from the loaded configuration.
fn apply_config(settings: &AppConfig, config: &slint_config::Config) {
    settings.set_language(config.language.clone().unwrap_or_default().into());
    settings.set_auto_update(config.auto_update);
    settings.set_update_channel(config.update_channel.as_str().into());

    settings.set_launch_width(config.launch.width.to_string().into());
    settings.set_launch_height(config.launch.height.to_string().into());
    settings.set_launch_fullscreen(config.launch.fullscreen);
    settings.set_quit_after_launch(config.launch.quit_app_after_launch);
    settings.set_auto_memory(config.launch.auto_memory);
    settings.set_max_memory(config.launch.max_memory.to_string().into());
    settings.set_gc(gc_to_str(&config.launch.gc).into());
    settings.set_extra_jvm_args(config.launch.extra_jvm_args.clone().into());
    settings.set_extra_mc_args(config.launch.extra_mc_args.clone().into());
    settings.set_extra_class_paths(config.launch.extra_class_paths.clone().into());
    settings.set_before_launch(config.launch.execute_before_launch.clone().into());
    settings.set_wrap_command(config.launch.wrap_command.clone().into());
    settings.set_after_launch(config.launch.execute_after_launch.clone().into());
    settings.set_ignore_invalid_certs(config.launch.ignore_invalid_minecraft_certificates);
    settings.set_ignore_patch_discrepancies(config.launch.ignore_patch_discrepancies);
    settings.set_skip_file_check(config.launch.skip_check_files);

    settings.set_prefer_mojang_java(config.prefer_mojang_java);

    settings.set_palette_follow_system(config.appearance.palette_follow_system);
    settings.set_palette(config.appearance.palette.clone().into());
    settings.set_background_image(
        config
            .appearance
            .background_image
            .clone()
            .unwrap_or_default()
            .into(),
    );
    settings.set_background_darkness(config.appearance.background_darkness as i32);
    settings.set_background_camera_move(config.appearance.background_camera_move);
    settings.set_background_parallax(config.appearance.background_parallax);

    settings.set_music_enabled(config.music.enabled);
    settings.set_resume_on_startup(config.music.resume_on_startup);
    settings.set_show_visualizer(config.music.show_visualizer);
    settings.set_pause_on_launch(config.music.pause_on_launch);
    settings.set_main_volume(config.music.main_volumn as i32);
    settings.set_background_volume(config.music.main_volumn_background as i32);

    settings.set_max_connections(config.download.max_connections.to_string().into());
    settings.set_max_download_speed(config.download.max_download_speed.to_string().into());
    settings.set_use_system_proxy(config.download.use_system_proxy);

    settings.set_release_reminder(config.accessibility.release_reminder);
    settings.set_snapshot_reminder(config.accessibility.snapshot_reminder);
    settings.set_auto_game_lang(config.accessibility.change_game_language);
    settings.set_high_contrast(config.accessibility.high_contrast_mode);

    settings.set_app_version(env!("CARGO_PKG_VERSION").into());
}

fn parse_usize_keep(value: &str, previous: usize) -> usize {
    value.trim().parse().unwrap_or(previous)
}

fn parse_u64_keep(value: &str, previous: u64) -> u64 {
    value.trim().parse().unwrap_or(previous)
}

fn parse_u8_keep(value: i32, previous: u8) -> u8 {
    u8::try_from(value).unwrap_or(previous)
}

/// Reads the Slint `AppConfig` global back into a configuration, keeping the
/// previous values for unparseable numbers and for state not represented in
/// the UI (the Java disabled list is driven by `set-java-enabled`).
fn collect_config(settings: &AppConfig, mut config: slint_config::Config) -> slint_config::Config {
    let language = settings.get_language().to_string();
    config.language = if language.is_empty() {
        None
    } else {
        Some(language)
    };
    config.auto_update = settings.get_auto_update();
    config.update_channel = slint_config::UpdateChannel::from_slug(&settings.get_update_channel());

    config.launch.width = parse_usize_keep(&settings.get_launch_width(), config.launch.width);
    config.launch.height = parse_usize_keep(&settings.get_launch_height(), config.launch.height);
    config.launch.fullscreen = settings.get_launch_fullscreen();
    config.launch.quit_app_after_launch = settings.get_quit_after_launch();
    config.launch.auto_memory = settings.get_auto_memory();
    config.launch.max_memory =
        parse_usize_keep(&settings.get_max_memory(), config.launch.max_memory);
    config.launch.gc = gc_from_str(&settings.get_gc());
    config.launch.extra_jvm_args = settings.get_extra_jvm_args().to_string();
    config.launch.extra_mc_args = settings.get_extra_mc_args().to_string();
    config.launch.extra_class_paths = settings.get_extra_class_paths().to_string();
    config.launch.execute_before_launch = settings.get_before_launch().to_string();
    config.launch.wrap_command = settings.get_wrap_command().to_string();
    config.launch.execute_after_launch = settings.get_after_launch().to_string();
    config.launch.ignore_invalid_minecraft_certificates = settings.get_ignore_invalid_certs();
    config.launch.ignore_patch_discrepancies = settings.get_ignore_patch_discrepancies();
    config.launch.skip_check_files = settings.get_skip_file_check();

    config.prefer_mojang_java = settings.get_prefer_mojang_java();

    config.appearance.palette_follow_system = settings.get_palette_follow_system();
    config.appearance.palette = settings.get_palette().to_string();
    let background = settings.get_background_image().to_string();
    config.appearance.background_image = if background.is_empty() {
        None
    } else {
        Some(background)
    };
    config.appearance.background_darkness = parse_u8_keep(
        settings.get_background_darkness(),
        config.appearance.background_darkness,
    );
    config.appearance.background_camera_move = settings.get_background_camera_move();
    config.appearance.background_parallax = settings.get_background_parallax();

    config.music.enabled = settings.get_music_enabled();
    config.music.resume_on_startup = settings.get_resume_on_startup();
    config.music.show_visualizer = settings.get_show_visualizer();
    config.music.pause_on_launch = settings.get_pause_on_launch();
    config.music.main_volumn = parse_u8_keep(settings.get_main_volume(), config.music.main_volumn);
    config.music.main_volumn_background = parse_u8_keep(
        settings.get_background_volume(),
        config.music.main_volumn_background,
    );

    config.download.max_connections = parse_usize_keep(
        &settings.get_max_connections(),
        config.download.max_connections,
    );
    config.download.max_download_speed = parse_u64_keep(
        &settings.get_max_download_speed(),
        config.download.max_download_speed,
    );
    config.download.use_system_proxy = settings.get_use_system_proxy();

    config.accessibility.release_reminder = settings.get_release_reminder();
    config.accessibility.snapshot_reminder = settings.get_snapshot_reminder();
    config.accessibility.change_game_language = settings.get_auto_game_lang();
    config.accessibility.high_contrast_mode = settings.get_high_contrast();

    config
}

fn gc_to_str(gc: &slint_config::launch::GC) -> &'static str {
    match gc {
        slint_config::launch::GC::Z => "Z",
        slint_config::launch::GC::Parallel => "Parallel",
        slint_config::launch::GC::Serial => "Serial",
        slint_config::launch::GC::G1 => "G1",
    }
}

fn gc_from_str(value: &str) -> slint_config::launch::GC {
    match value {
        "Z" => slint_config::launch::GC::Z,
        "Parallel" => slint_config::launch::GC::Parallel,
        "Serial" => slint_config::launch::GC::Serial,
        _ => slint_config::launch::GC::G1,
    }
}

/// Builds the Java runtime model, marking the paths present in `disabled`.
fn java_model(runtimes: &[java::DetectedJava], disabled: &[String]) -> ModelRc<JavaRuntime> {
    let rows: Vec<JavaRuntime> = runtimes
        .iter()
        .map(|runtime| {
            let path = runtime.path_string();
            JavaRuntime {
                major: runtime.major,
                vendor: SharedString::from(runtime.vendor.clone()),
                version: SharedString::from(runtime.version.clone()),
                arch: SharedString::from(runtime.arch.clone()),
                enabled: !disabled.iter().any(|disabled| disabled == &path),
                path: SharedString::from(path),
            }
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// Opens a URL or a local path with the platform's default handler.
fn open_external(target: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");

    command.arg(target);
    command.spawn().map(|_| ())
}

/// Native "choose an image file" dialog (no extra dependency).
fn pick_image_file() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .args([
                "-e",
                "POSIX path of (choose file with prompt \"Select an image\")",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return (!path.is_empty()).then(|| PathBuf::from(path));
    }
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--file-filter=Images | *.png *.jpg *.jpeg *.webp *.gif *.bmp *.avif *.svg *.ico",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return (!path.is_empty()).then(|| PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    {
        let script = "Add-Type -AssemblyName System.Windows.Forms; \
            $f = New-Object System.Windows.Forms.OpenFileDialog; \
            $f.Filter = 'Images|*.png;*.jpg;*.jpeg;*.webp;*.gif;*.bmp;*.avif;*.svg;*.ico'; \
            if ($f.ShowDialog() -eq 'OK') { $f.FileName }";
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return (!path.is_empty()).then(|| PathBuf::from(path));
    }
    #[allow(unreachable_code)]
    None
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

    // Configuration (shared with the Tauri app).
    let config = slint_config::load_config_file().unwrap_or_else(|error| {
        log::error!("failed to load config: {error}");
        slint_config::Config::default()
    });

    // Pick the bundled translation. Must run after a component exists (that's
    // what installs the translation bundle).
    select_locale(config.language.as_deref());

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
    apply_config(&settings, &config);

    let shared = Rc::new(RefCell::new(config));
    let save_timer = Rc::new(Timer::default());
    // `CONIC_LOCALE` forces the translation and disables runtime switching.
    let forced_locale = std::env::var("CONIC_LOCALE").is_ok();

    // Debounced persistence: editing a text field fires `changed` per keypress.
    {
        let shared = Rc::clone(&shared);
        let save_timer = Rc::clone(&save_timer);
        let weak = ui.as_weak();
        settings.on_changed(move || {
            let mut language_changed = None;
            if let Some(ui) = weak.upgrade() {
                let settings = ui.global::<AppConfig>();
                let mut config = shared.borrow_mut();
                let old_language = config.language.clone();
                *config = collect_config(&settings, config.clone());
                if !forced_locale && config.language != old_language {
                    language_changed = Some(config.language.clone().unwrap_or_default());
                }
            }
            // Slint re-evaluates every `@tr` binding after this call, so the UI
            // switches language without a restart.
            if let Some(language) = language_changed {
                apply_locale(resolve_locale(&language));
            }
            let shared = Rc::clone(&shared);
            save_timer.start(
                TimerMode::SingleShot,
                Duration::from_millis(400),
                move || {
                    let config = shared.borrow();
                    if let Err(error) = slint_config::save_config(&config) {
                        log::error!("failed to save config: {error}");
                    }
                },
            );
        });
    }

    // OS integrations (settings callbacks).
    settings.on_open_url(move |url| {
        if let Err(error) = open_external(url.as_str()) {
            log::warn!("failed to open url '{url}': {error}");
        }
    });
    {
        settings.on_open_path(move |key| {
            let path = match key.as_str() {
                "music" => slint_config::music_dir(),
                "logs" => slint_config::logs_dir(),
                other => PathBuf::from(other.to_string()),
            };
            if let Err(error) = open_external(&path.to_string_lossy()) {
                log::warn!("failed to open path '{}': {error}", path.display());
            }
        });
    }
    {
        let shared = Rc::clone(&shared);
        let save_timer = Rc::clone(&save_timer);
        let weak = ui.as_weak();
        settings.on_pick_background_image(move || {
            let Some(path) = pick_image_file() else {
                return;
            };
            match slint_config::set_background_image(&path) {
                Ok(filename) => {
                    let mut config = shared.borrow_mut();
                    config.appearance.background_image = Some(filename.clone());
                    drop(config);
                    if let Some(ui) = weak.upgrade() {
                        ui.global::<AppConfig>()
                            .set_background_image(filename.into());
                    }
                    let shared = Rc::clone(&shared);
                    save_timer.start(
                        TimerMode::SingleShot,
                        Duration::from_millis(50),
                        move || {
                            let config = shared.borrow();
                            let _ = slint_config::save_config(&config);
                        },
                    );
                }
                Err(error) => log::error!("failed to set background image: {error}"),
            }
        });
    }
    {
        let shared = Rc::clone(&shared);
        let weak = ui.as_weak();
        settings.on_remove_background_image(move || {
            if let Err(error) = slint_config::remove_background_image() {
                log::warn!("failed to remove background image: {error}");
            }
            if let Some(ui) = weak.upgrade() {
                ui.global::<AppConfig>()
                    .set_background_image(SharedString::default());
            }
            shared.borrow_mut().appearance.background_image = None;
            let config = shared.borrow();
            let _ = slint_config::save_config(&config);
        });
    }

    // Java runtime management.
    {
        let shared = Rc::clone(&shared);
        let weak = ui.as_weak();
        settings.on_rescan_java(move || {
            let disabled = shared.borrow().disabled_java_runtime.clone();
            if let Some(ui) = weak.upgrade() {
                let settings = ui.global::<AppConfig>();
                settings.set_java_scanning(true);
                settings.set_java_scan_error(SharedString::default());
            }
            let weak = weak.clone();
            let managed = slint_config::data_root().join("runtime");
            std::thread::spawn(move || {
                let runtimes = java::scan(&managed);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let settings = ui.global::<AppConfig>();
                    settings.set_java_runtimes(java_model(&runtimes, &disabled));
                    settings.set_java_scanning(false);
                    settings.set_java_scan_error(SharedString::default());
                    JAVA_CACHE.with(|cache| *cache.borrow_mut() = runtimes);
                });
            });
        });
    }
    {
        let shared = Rc::clone(&shared);
        let weak = ui.as_weak();
        settings.on_set_java_enabled(move |path, enabled| {
            let path = path.to_string();
            {
                let mut config = shared.borrow_mut();
                if enabled {
                    config.disabled_java_runtime.retain(|p| p != &path);
                } else if !config.disabled_java_runtime.iter().any(|p| p == &path) {
                    config.disabled_java_runtime.push(path);
                }
            }
            if let Some(ui) = weak.upgrade() {
                let settings = ui.global::<AppConfig>();
                let disabled = shared.borrow().disabled_java_runtime.clone();
                let model = JAVA_CACHE.with(|cache| java_model(&cache.borrow(), &disabled));
                settings.set_java_runtimes(model);
            }
            let config = shared.borrow();
            let _ = slint_config::save_config(&config);
        });
    }

    // TODO(migration): wire these to the real stores/panels.
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
