// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The settings "script": wires the `AppConfig` global callbacks (src/store/config.ts
//! + the settings screens) to the config crate and the OS integrations.

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::slint_backend::{App, AppConfig, GameState, JavaRuntime};
use crate::{config_bridge, java};

thread_local! {
    /// Last Java scan result. Only accessed on the UI thread so it can stay an
    /// `Rc`-free, `Send`-free `RefCell` while the scan itself runs on a worker.
    static JAVA_CACHE: RefCell<Vec<java::DetectedJava>> = const { RefCell::new(Vec::new()) };
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

/// Registers every settings callback on the `AppConfig` global.
pub fn wire(ui: &App, shared: Rc<RefCell<slint_config::Config>>, save_timer: Rc<Timer>) {
    let settings = ui.global::<AppConfig>();
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
                *config = config_bridge::collect_config(&settings, config.clone());
                if !forced_locale && config.language != old_language {
                    language_changed = Some(config.language.clone().unwrap_or_default());
                }
                // The content count suffix is locale-dependent, not translated.
                ui.global::<GameState>()
                    .set_count_unit(config_bridge::count_unit(config.language.as_deref()).into());
            }
            // Slint re-evaluates every `@tr` binding after this call, so the UI
            // switches language without a restart.
            if let Some(language) = language_changed {
                config_bridge::apply_locale(config_bridge::resolve_locale(&language));
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

    settings.on_open_url(move |url| {
        if let Err(error) = config_bridge::open_external(url.as_str()) {
            log::warn!("failed to open url '{url}': {error}");
        }
    });

    settings.on_open_path(move |key| {
        let path = match key.as_str() {
            "music" => slint_folder::DATA_LOCATION.music.clone(),
            "logs" => slint_folder::DATA_LOCATION.logs.clone(),
            other => PathBuf::from(other.to_string()),
        };
        if let Err(error) = config_bridge::open_external(&path.to_string_lossy()) {
            log::warn!("failed to open path '{}': {error}", path.display());
        }
    });

    {
        let shared = Rc::clone(&shared);
        let save_timer = Rc::clone(&save_timer);
        let weak = ui.as_weak();
        settings.on_pick_background_image(move || {
            let Some(path) = config_bridge::pick_image_file() else {
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
            let managed = slint_folder::DATA_LOCATION.runtime.clone();
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
}
