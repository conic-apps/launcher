// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The settings "script": translating between the persisted [`slint_config::Config`]
//! and the Slint `AppConfig` global (src/store/config.ts + the settings screens).
//! Also hosts the OS integration helpers used by the settings callbacks.

use std::path::PathBuf;

use crate::slint_backend::AppConfig;

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
        bundled_locale(slint_config::get_system_language())
    } else {
        bundled_locale(language)
    }
    .unwrap_or("en_US")
}

pub fn apply_locale(locale: &str) {
    if let Err(error) = slint::select_bundled_translation(locale) {
        log::warn!("failed to select locale '{locale}': {error}");
    } else {
        log::debug!(target: "app", "selected locale '{locale}'");
    }
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
        _ => bundled_locale(slint_config::get_system_language()),
    }
    .unwrap_or("en_US");
    apply_locale(locale);
}

/// Seeds the Slint `AppConfig` global from the loaded configuration.
pub fn apply_config(settings: &AppConfig, config: &slint_config::Config) {
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
        .unwrap_or_else(|| slint_config::get_system_language().to_string())
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
pub fn collect_config(
    settings: &AppConfig,
    mut config: slint_config::Config,
) -> slint_config::Config {
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

pub fn gc_to_str(gc: &slint_config::launch::GC) -> &'static str {
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

/// Native "choose an image file" dialog (no extra dependency).
pub fn pick_image_file() -> Option<PathBuf> {
    pick_image_file_named("Select an image")
}

/// The same picker with the filter label the caller wants to show. The Vue
/// passes a translated name here (`overlays.dialogs.createInstance.imagesFilter`
/// in the create-instance dialog), and the file types match the filter it uses.
pub fn pick_image_file_named(name: &str) -> Option<PathBuf> {
    /// The file types the Vue's filter offers. macOS takes only the prompt, so
    /// the pattern list is unused there.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    const EXTENSIONS: &str = "*.png *.jpg *.jpeg *.webp *.gif *.bmp *.avif *.svg *.ico";
    #[cfg(target_os = "macos")]
    {
        // The label becomes the dialog's prompt: AppleScript strings need their
        // quotes and backslashes escaped.
        let prompt = name.replace('\\', "\\\\").replace('"', "\\\"");
        let output = std::process::Command::new("osascript")
            .args([
                "-e",
                &format!("POSIX path of (choose file with prompt \"{prompt}\")"),
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
                &format!("--file-filter={name} | {EXTENSIONS}"),
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
        // The filter string separates the label from the patterns with a `|`.
        let filter = format!("{}|{}", name, EXTENSIONS.replace(' ', ";"));
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
            $f = New-Object System.Windows.Forms.OpenFileDialog; \
            $f.Filter = '{filter}'; \
            if ($f.ShowDialog() -eq 'OK') {{ $f.FileName }}"
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
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
