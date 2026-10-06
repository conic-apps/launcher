// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The settings "script": translating between the persisted [`config::Config`]
//! and the Slint `AppConfig` global. Also hosts the OS integration helpers used
//! by the settings callbacks.

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, RwLock};

use crate::slint_backend::AppConfig;

/// The catalog [`apply_locale`] last selected, for the things that have to agree
/// with the text rather than with the config (see [`active_language_tag`]).
///
/// A lock rather than a `OnceLock`, because a language change calls
/// `apply_locale` again and the last one has to win. It starts empty, which
/// means "not chosen yet" — the read below then falls back to the system
/// locale, and a *read* never pins the value.
static APPLIED_LOCALE: LazyLock<RwLock<String>> = LazyLock::new(|| RwLock::new(String::new()));

/// Maps a launcher language code to the bundled gettext locale (the catalog
/// folder name). All 12 launcher languages ship a catalog.
pub fn bundled_locale(code: &str) -> Option<&'static str> {
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
pub fn resolve_locale(language: &str) -> &'static str {
    if language.is_empty() {
        bundled_locale(config::get_system_language())
    } else {
        bundled_locale(language)
    }
    .unwrap_or("en_US")
}

/// The catalog that is actually in effect, as a BCP 47 tag.
///
/// For markup that is not a Slint UI — the `<html lang>` of the OAuth callback
/// page. The tag follows the *applied* catalog rather than the config's
/// `language`, because those are two different things: `select_locale` honours
/// `CONIC_LOCALE` and prefers the system locale when the config is empty, and a
/// document whose sentences are in one language and whose `lang` says another
/// is a document a screen reader and a hyphenator get wrong.
pub fn active_language_tag() -> String {
    let applied = APPLIED_LOCALE
        .read()
        .map(|locale| locale.clone())
        .unwrap_or_default();
    if applied.is_empty() {
        resolve_locale("")
    } else {
        &applied
    }
    .replace('_', "-")
}

pub fn apply_locale(locale: &str) {
    if let Err(error) = slint::select_bundled_translation(locale) {
        log::warn!("failed to select locale '{locale}': {error}");
    } else {
        log::debug!(target: "app", "selected locale '{locale}'");
        // Only remembered on success: a catalog that would not load is not the
        // one a document should be tagged with.
        if let Ok(mut applied) = APPLIED_LOCALE.write() {
            *applied = locale.to_string();
        }
    }
    // The catalog only carries text; which font draws the Han characters in it
    // is a separate, also locale-dependent choice (see `native`). Kept here so
    // a language change picks up both.
    crate::support::native::apply_cjk_fallbacks(locale);
}

/// Selects the bundled translation that best matches the preferred language
/// (from the config) or the system locale. `CONIC_LOCALE` overrides both and
/// disables runtime switching.
pub fn select_locale(preferred: Option<&str>) {
    if let Ok(override_locale) = std::env::var("CONIC_LOCALE") {
        apply_locale(&override_locale);
        return;
    }
    let locale = match preferred {
        Some(language) if !language.is_empty() => bundled_locale(language),
        _ => bundled_locale(config::get_system_language()),
    }
    .unwrap_or("en_US");
    apply_locale(locale);
}

/// Seeds the Slint `AppConfig` global from the loaded configuration.
pub fn apply_config(settings: &AppConfig, config: &config::Config) {
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

    // The storage roots come from the bootstrap file rather than the config, so
    // they are seeded from the already-resolved layout. The settings page shows
    // them and opens them, and the Change buttons write an override.
    settings.set_launcher_location(
        storage::LOCATIONS
            .launcher
            .root
            .to_string_lossy()
            .to_string()
            .into(),
    );
    settings.set_minecraft_location(
        storage::LOCATIONS
            .minecraft
            .root
            .to_string_lossy()
            .to_string()
            .into(),
    );
    settings.set_instances_location(
        storage::LOCATIONS
            .instances
            .root
            .to_string_lossy()
            .to_string()
            .into(),
    );
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

/// The launcher language code actually in effect: `CONIC_LOCALE` overrides the
/// configured language, which falls back to the system locale.
pub fn active_language_code(configured: Option<&str>) -> String {
    std::env::var("CONIC_LOCALE")
        .ok()
        .map(|locale| locale.to_lowercase().replace('-', "_"))
        .or_else(|| {
            configured
                .map(str::to_string)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| config::get_system_language().to_string())
}

/// The suffix shown after content counts. Only Chinese uses one; it is derived
/// from the language rather than a gettext key (the English source is empty).
pub fn count_unit(configured: Option<&str>) -> &'static str {
    match active_language_code(configured).as_str() {
        "zh_cn" | "zh_hans" => "个",
        "zh_tw" | "zh_hant" | "zh_hk" => "個",
        _ => "",
    }
}

/// Reads the Slint `AppConfig` global back into a configuration, keeping the
/// previous values for unparseable numbers and for state not represented in
/// the UI (the Java disabled list is driven by `set-java-enabled`).
pub fn collect_config(settings: &AppConfig, mut config: config::Config) -> config::Config {
    let language = settings.get_language().to_string();
    config.language = if language.is_empty() {
        None
    } else {
        Some(language)
    };
    config.auto_update = settings.get_auto_update();
    config.update_channel = config::UpdateChannel::from_slug(&settings.get_update_channel());

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

pub fn gc_to_str(gc: &config::launch::GC) -> &'static str {
    match gc {
        config::launch::GC::Z => "Z",
        config::launch::GC::Parallel => "Parallel",
        config::launch::GC::Serial => "Serial",
        config::launch::GC::G1 => "G1",
    }
}

pub(crate) fn gc_from_str(value: &str) -> config::launch::GC {
    match value {
        "Z" => config::launch::GC::Z,
        "Parallel" => config::launch::GC::Parallel,
        "Serial" => config::launch::GC::Serial,
        _ => config::launch::GC::G1,
    }
}

/// Opens a URL or a local path with the platform's default handler.
pub fn open_external(target: &str) -> std::io::Result<()> {
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

/// Opens the file manager with `path` selected.
///
/// On macOS `open -R`, on Windows `explorer /select,`, and on Linux the
/// `org.freedesktop.FileManager1` D-Bus call (with the parent directory as the
/// fallback, since a desktop without that service cannot select a file).
/// `open_external` is *not* a substitute — it launches the file, it does not
/// show it in its folder.
pub fn reveal_in_dir(path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", path])
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn()
            .map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // `FileManager1.ShowItems` takes a list of URIs.
        let uri = format!("file://{}", path.replace(' ', "%20"));
        let shown = std::process::Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.freedesktop.FileManager1",
                "--type=method_call",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems",
            ])
            .arg(format!("array:string:{uri}"))
            .arg("string:")
            .spawn()
            .map(|_| ());
        if shown.is_ok() {
            return shown;
        }
        // No FileManager1 on this desktop: at least open the folder.
        let parent = std::path::Path::new(path)
            .parent()
            .map(|parent| parent.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        open_external(&parent)
    }
}

/// The one system clipboard handle, opened lazily and kept for the life of the
/// app by [`copy_to_clipboard`] and released by [`drop_clipboard`].
///
/// On Linux the clipboard is not a data store: the process that last copied the
/// text owns the selection and answers paste requests until another app takes
/// over. Opening a `Clipboard` inside `copy_to_clipboard` and letting it drop on
/// return would end the selection before the user could paste — the failure the
/// old `wl-copy`/`xclip` calls avoided by daemonising. One handle held here
/// reproduces that, and is the arrangement `arboard` recommends for a
/// long-running GUI. `Mutex::new` is `const`, so no lazy initializer is needed.
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// Writes `text` to the system clipboard.
///
/// Slint 1.18 has no application-level clipboard API — `Platform::set_clipboard_text`
/// is only reachable from inside a backend, which the app is not — so the app's
/// "copy" buttons go to the platform directly, the way `open_external` above
/// already does. `arboard` covers what the old per-platform `NSPasteboard` /
/// `clip` / `wl-copy`+`xclip` branches did in one path, and reaches Wayland
/// through its `wayland-data-control` feature.
pub fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    // A poisoned lock only means a previous copy panicked mid-write; the
    // clipboard is still safe to use, so take the guard back.
    let mut clipboard = CLIPBOARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if clipboard.is_none() {
        // `Clipboard::new` can fail where there is no display or clipboard
        // manager. Leave the handle empty and retry on the next call rather
        // than caching the failure for the life of the process.
        *clipboard = arboard::Clipboard::new().ok();
    }
    clipboard
        .as_mut()
        .ok_or_else(|| std::io::Error::other("could not open the system clipboard"))?
        .set_text(text)
        .map_err(|error| std::io::Error::other(error.to_string()))
}

/// Releases the clipboard handle [`copy_to_clipboard`] opened.
///
/// `arboard` keeps a background thread on some platforms and asks that the
/// handle be dropped before the process exits; the Slint/winit event loop does
/// not drop a process-global on its own, so `main` calls this once `ui.run`
/// returns.
pub fn drop_clipboard() {
    CLIPBOARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
}

/// The image extensions the pickers filter on, without the leading dot: `rfd`
/// wants bare extensions and builds each platform's own pattern list from them.
const IMAGE_EXTENSIONS: [&str; 9] = [
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg", "ico",
];

/// Native "choose an image file" dialog.
pub fn pick_image_file() -> Option<PathBuf> {
    pick_image_file_named("Select an image")
}

/// The same picker with the filter label the caller wants to show. The caller
/// passes a translated name here (`@tr("CreateInstance" => "Images")` in the
/// create-instance dialog, for example), and the file types match the filter it
/// uses.
pub fn pick_image_file_named(name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(name, &IMAGE_EXTENSIONS)
        .pick_file()
}

/// Native "save a file as" dialog.
///
/// The account view's "save my skin" is the one caller.
pub fn save_file_named(prompt: &str, default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(prompt)
        .set_file_name(default_name)
        .save_file()
}

/// Native "choose a folder" dialog, opened at `current`.
pub fn pick_directory(prompt: &str, current: &Path) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(prompt)
        .set_directory(current)
        .pick_folder()
}

/// Re-executes the launcher so a changed storage location takes effect.
///
/// The child carries `CONIC_RELAUNCH`, which tells `main` to wait for this
/// process' single-instance claim to be released before trying to take it: the
/// parent is still shutting down when the child starts, and without that wait
/// the child would see `AlreadyRunning` and exit, losing the restart.
pub fn relaunch() {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            log::error!("failed to restart: {error}");
            return;
        }
    };
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match std::process::Command::new(exe)
        .args(args)
        .env("CONIC_RELAUNCH", "1")
        .spawn()
    {
        // The child waits for this process to exit, which `quit_event_loop`
        // brings about after `ui.run` returns and the cleanup runs.
        Ok(_) => {
            if let Err(error) = slint::quit_event_loop() {
                log::error!("failed to stop the running instance: {error}");
            }
        }
        Err(error) => log::error!("failed to restart: {error}"),
    }
}
