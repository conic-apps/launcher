// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance settings overlay's "script" (src/overlays/InstanceSetting.vue):
//! it owns the instance the overlay is editing, turns the UI's edits into an
//! `InstanceConfig` and writes `instance.toml` — the work the Vue's
//! `watchEffect` did on every edit.
//!
//! The delete dialog the overlay opens (src/overlays/dialogs/
//! ConfirmDeleteInstance.vue) is wired here too, since it is handed the same
//! instance.

use std::{cell::RefCell, rc::Rc, time::Duration};

use slint::{ComponentHandle, Image, SharedPixelBuffer, Timer, TimerMode};

use crate::config_bridge;
use crate::game::format_play_time;
use crate::slint_backend::{App, DeleteInstanceState, Dialogs, GameState, InstanceSettingsState};
use slint_config::launch::{LaunchConfig, Server};
use slint_instance::{Instance, InstanceConfig, InstanceLaunchConfig};

/// How long an edit waits before it reaches `instance.toml`.
///
/// The Vue writes on every keystroke and blocks the whole app while the write is
/// in flight (`document.body.classList.add("saving-instance-settings")`, which
/// `main.css` turns into `pointer-events: none !important`). A Slint write is a
/// synchronous call on the UI thread with no IPC in front of it, so there is no
/// window for a second edit to slip into and nothing to lock; what is left is
/// the cost of one `toml` write per keystroke, which the 400 ms the settings
/// screen already uses absorbs.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

/// Opens the overlay on the current instance (`useInstanceSettings().value =
/// true`, from the game view's settings button).
pub fn open(ui: &App) {
    let Some(instance) = current_instance(ui) else {
        log::warn!("the instance settings were opened without a current instance");
        return;
    };
    let state = ui.global::<InstanceSettingsState>();
    apply(&state, &instance);
    state.set_background(load_background(&instance));
    state.set_visible(true);
}

/// The instance the overlay is editing: the game view's current one, which cannot
/// change while the overlay covers it.
fn current_instance(ui: &App) -> Option<Instance> {
    let id = ui.global::<GameState>().get_current_id();
    if id.is_empty() {
        return None;
    }
    slint_instance::get_instance_by_id(id.as_str())
}

/// Puts an instance into the overlay's fields.
fn apply(state: &InstanceSettingsState, instance: &Instance) {
    state.set_name(instance.config.name.clone().into());
    state.set_loader(loader_name(instance).into());
    state.set_minecraft_version(instance.config.runtime.minecraft.clone().into());
    state.set_has_background(instance.has_background);
    state.set_name_editing(false);
    apply_launch(state, &instance.config);
}

/// Puts an instance's launch settings into the overlay's fields.
///
/// Every number is a string because `TextInput` edits text (see
/// `globals/instance-settings.slint`); an `Option` the instance never set is an
/// empty field, which is what the Vue's `<input>` shows its placeholder in.
fn apply_launch(state: &InstanceSettingsState, config: &InstanceConfig) {
    let launch = &config.launch_config;
    state.set_use_as_launcher_background(config.use_as_launcher_background);
    state.set_enable_specific(launch.enable_instance_specific_settings);
    state.set_width(number(launch.width).into());
    state.set_height(number(launch.height).into());
    state.set_fullscreen(launch.fullscreen.unwrap_or(false));
    state.set_quit_after_launch(launch.quit_app_after_launch.unwrap_or(false));
    state.set_skip_file_check(launch.skip_check_files.unwrap_or(false));
    state.set_auto_memory(launch.auto_memory.unwrap_or(false));
    state.set_max_memory(number(launch.max_memory).into());
    // An unset collector is an empty string rather than the default one: the
    // Vue's select shows no option highlighted in that case, and its "is this
    // still the default?" test is `gc === "G1"`, which an unset `gc` fails.
    state.set_gc(match launch.gc {
        Some(ref gc) => config_bridge::gc_to_str(gc).into(),
        None => "".into(),
    });
    state.set_extra_jvm_args(launch.extra_jvm_args.clone().unwrap_or_default().into());
    state.set_extra_mc_args(launch.extra_mc_args.clone().unwrap_or_default().into());
    state.set_extra_class_paths(launch.extra_class_paths.clone().unwrap_or_default().into());
    state.set_before_launch(
        launch
            .execute_before_launch
            .clone()
            .unwrap_or_default()
            .into(),
    );
    state.set_wrap_command(launch.wrap_command.clone().unwrap_or_default().into());
    state.set_after_launch(
        launch
            .execute_after_launch
            .clone()
            .unwrap_or_default()
            .into(),
    );
    state.set_ignore_invalid_certs(
        launch
            .ignore_invalid_minecraft_certificates
            .unwrap_or(false),
    );
    state.set_ignore_patch_discrepancies(launch.ignore_patch_discrepancies.unwrap_or(false));
}

/// An optional number as the field that edits it.
fn number(value: Option<usize>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// A field's text as the number it names.
///
/// A blank field is `None`, which clears the setting — the Vue's `v-model.number`
/// leaves a blank field blank rather than making it zero. Anything else that does
/// not parse keeps `previous`, the way `config_bridge::parse_usize_keep` does for
/// the settings screen: a field mid-edit is briefly unparseable, and a save must
/// not throw the setting away over it.
fn parse_number(value: &str, previous: Option<usize>) -> Option<usize> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.parse().unwrap_or_else(|_| previous.unwrap_or(0)))
}

/// The instance's config with the overlay's edits merged into it.
///
/// Only what the overlay shows is read back: the runtime, the group and the
/// starred flag are taken from the config that was just re-read from disk, so
/// nothing the UI does not touch can be lost through it.
fn collect(state: &InstanceSettingsState, mut config: InstanceConfig) -> InstanceConfig {
    config.name = state.get_name().to_string();
    config.use_as_launcher_background = state.get_use_as_launcher_background();
    let gc = state.get_gc();
    // Taken rather than borrowed, so the fields the overlay does not show can be
    // moved out of it and into the replacement below.
    let mut previous = std::mem::take(&mut config.launch_config);
    let launch = InstanceLaunchConfig {
        enable_instance_specific_settings: state.get_enable_specific(),
        // Not a field of the Vue's TypeScript type, and the Vue replaces the
        // whole `launch_config` on every write — which silently drops it. The
        // launcher reads it (`instance_java_path`), so it is carried over here
        // instead of cleared.
        java_path: previous.java_path.take(),
        auto_memory: Some(state.get_auto_memory()),
        max_memory: parse_number(state.get_max_memory().as_str(), previous.max_memory),
        server: previous.server.take(),
        width: parse_number(state.get_width().as_str(), previous.width),
        height: parse_number(state.get_height().as_str(), previous.height),
        fullscreen: Some(state.get_fullscreen()),
        extra_jvm_args: Some(state.get_extra_jvm_args().to_string()),
        extra_mc_args: Some(state.get_extra_mc_args().to_string()),
        is_demo: previous.is_demo.take(),
        ignore_invalid_minecraft_certificates: Some(state.get_ignore_invalid_certs()),
        ignore_patch_discrepancies: Some(state.get_ignore_patch_discrepancies()),
        extra_class_paths: Some(state.get_extra_class_paths().to_string()),
        gc: if gc.is_empty() {
            previous.gc.take()
        } else {
            Some(config_bridge::gc_from_str(gc.as_str()))
        },
        launcher_name: previous.launcher_name.take(),
        wrap_command: Some(state.get_wrap_command().to_string()),
        execute_before_launch: Some(state.get_before_launch().to_string()),
        execute_after_launch: Some(state.get_after_launch().to_string()),
        skip_check_files: Some(state.get_skip_file_check()),
        quit_app_after_launch: Some(state.get_quit_after_launch()),
    };
    config.launch_config = launch;
    config
}

/// Whether the game view shows something an edit changed: the instance's name
/// (the summary's title and its list card) and the two flags the window
/// background resolves on. Everything else the overlay edits is read when the
/// game is launched.
fn touches_game_view(before: &InstanceConfig, after: &InstanceConfig) -> bool {
    before.name != after.name
        || before.use_as_launcher_background != after.use_as_launcher_background
}

/// Schedules the write of an edited instance config.
///
/// The instance travels with the write rather than being looked up when it fires:
/// the overlay can be closed — and the current instance changed — in between.
/// A run of keystrokes is therefore one `instance.toml`.
fn schedule_save(save_timer: &Rc<Timer>, ui: &App, config: InstanceConfig, touches: bool) {
    let Some(id) = current_instance(ui).map(|instance| instance.id) else {
        return;
    };
    let weak = ui.as_weak();
    let save_timer = Rc::clone(save_timer);
    // The timer takes an `FnMut`, so the config it carries is taken out of an
    // `Option` rather than moved — and starting an armed timer replaces the
    // callback, which is what makes a second edit within the debounce replace
    // the first rather than queue behind it.
    let mut pending = Some(config);
    save_timer.start(TimerMode::SingleShot, SAVE_DEBOUNCE, move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let Some(config) = pending.take() else {
            return;
        };
        if let Err(error) = slint_instance::update_instance(config, &id) {
            log::error!("failed to save the instance '{id}': {error}");
            return;
        }
        if touches {
            // The Vue edits the very object its Pinia store holds, so the
            // summary and the list card re-render off it for free. Here the
            // store is on disk, so the game view is asked to read it again.
            ui.global::<GameState>().invoke_refresh();
        }
    });
}

/// Registers every instance settings callback, and the delete dialog's.
pub fn setup(ui: &App, config: Rc<RefCell<slint_config::Config>>) {
    let state = ui.global::<InstanceSettingsState>();
    let save_timer = Rc::new(Timer::default());

    // Any field of the form.
    {
        let weak = ui.as_weak();
        let save_timer = Rc::clone(&save_timer);
        state.on_changed(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance(&ui) else {
                return;
            };
            let state = ui.global::<InstanceSettingsState>();
            let updated = collect(&state, instance.config.clone());
            let touches = touches_game_view(&instance.config, &updated);
            schedule_save(&save_timer, &ui, updated, touches);
        });
    }

    // `enable_instance_specific_settings`, the switch the launch options, the
    // memory group and the advanced options hang off.
    {
        let weak = ui.as_weak();
        let config = Rc::clone(&config);
        let save_timer = Rc::clone(&save_timer);
        state.on_set_specific(move |enabled| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance(&ui) else {
                return;
            };
            // Started from the overlay's own fields, not from what is on disk: a
            // rename typed a moment ago is still waiting on its debounce, and
            // this write replaces that one rather than queueing behind it.
            let state = ui.global::<InstanceSettingsState>();
            let mut updated = collect(&state, instance.config.clone());
            updated.launch_config = if enabled {
                inherit_launch_config(&config.borrow().launch)
            } else {
                InstanceLaunchConfig {
                    enable_instance_specific_settings: false,
                    ..InstanceLaunchConfig::default()
                }
            };
            // Both branches replaced every field, so the panels are pushed what
            // was stored rather than what was asked for.
            apply_launch(&state, &updated);
            let touches = touches_game_view(&instance.config, &updated);
            schedule_save(&save_timer, &ui, updated, touches);
        });
    }

    // `resetAdvanceOptions`: the nine advanced fields back to what a fresh config
    // holds, and nothing else — the window size, the memory and the file check
    // are left as they are.
    {
        let weak = ui.as_weak();
        let save_timer = Rc::clone(&save_timer);
        state.on_reset_advanced(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance(&ui) else {
                return;
            };
            let defaults = slint_config::Config::default().launch;
            let state = ui.global::<InstanceSettingsState>();
            // From the overlay's own fields, for the same reason the switch above
            // is: this write replaces whatever was waiting on the debounce.
            let mut updated = collect(&state, instance.config.clone());
            let launch = &mut updated.launch_config;
            launch.gc = Some(defaults.gc);
            launch.extra_jvm_args = Some(defaults.extra_jvm_args);
            launch.extra_mc_args = Some(defaults.extra_mc_args);
            launch.extra_class_paths = Some(defaults.extra_class_paths);
            launch.execute_before_launch = Some(defaults.execute_before_launch);
            launch.wrap_command = Some(defaults.wrap_command);
            launch.execute_after_launch = Some(defaults.execute_after_launch);
            launch.ignore_invalid_minecraft_certificates =
                Some(defaults.ignore_invalid_minecraft_certificates);
            launch.ignore_patch_discrepancies = Some(defaults.ignore_patch_discrepancies);
            apply_launch(&state, &updated);
            schedule_save(&save_timer, &ui, updated, false);
        });
    }

    // `getBackground()`: the native picker, the copy into the instance, and a
    // re-list so the window background picks the new file up.
    {
        let weak = ui.as_weak();
        state.on_pick_background(move |filter_name| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance(&ui) else {
                return;
            };
            let Some(path) = config_bridge::pick_image_file_named(filter_name.as_str()) else {
                return;
            };
            if let Err(error) = slint_instance::add_background_image(&path, &instance.id) {
                log::error!("failed to set the background of '{}': {error}", instance.id);
                return;
            }
            // `has_background` is the file's existence, so the instance has to be
            // read again rather than patched.
            let Some(instance) = slint_instance::get_instance_by_id(&instance.id) else {
                return;
            };
            let state = ui.global::<InstanceSettingsState>();
            state.set_has_background(instance.has_background);
            state.set_background(load_background(&instance));
            ui.global::<GameState>().invoke_refresh();
        });
    }

    // The red "Remove image" button, and the Vue's `removeBackground` +
    // `loadInstances()` after it.
    {
        let weak = ui.as_weak();
        state.on_remove_background(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance(&ui) else {
                return;
            };
            if let Err(error) = slint_instance::remove_background(&instance.id) {
                log::error!(
                    "failed to remove the background of '{}': {error}",
                    instance.id
                );
                return;
            }
            let state = ui.global::<InstanceSettingsState>();
            state.set_has_background(false);
            state.set_background(Image::default());
            ui.global::<GameState>().invoke_refresh();
        });
    }

    // `openDeleteInstanceDialog`, which closes this overlay first.
    {
        let weak = ui.as_weak();
        state.on_open_delete(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance(&ui) else {
                return;
            };
            ui.global::<InstanceSettingsState>().set_visible(false);
            open_delete_dialog(&ui, &instance);
        });
    }

    {
        let weak = ui.as_weak();
        state.on_close(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<InstanceSettingsState>().set_visible(false);
            }
        });
    }

    setup_delete_dialog(ui);
}

/// The instance's launch settings, seeded from the launcher's own.
///
/// This is the Vue's first `watchEffect` branch: an object literal naming every
/// field of the launcher's `config.launch` **except** `skip_refresh_account`,
/// which therefore goes back to the default. `java_path` has no counterpart
/// there — the field does not exist in the Vue's type at all.
fn inherit_launch_config(launcher: &LaunchConfig) -> InstanceLaunchConfig {
    InstanceLaunchConfig {
        enable_instance_specific_settings: true,
        java_path: None,
        auto_memory: Some(launcher.auto_memory),
        max_memory: Some(launcher.max_memory),
        server: launcher
            .server
            .as_ref()
            .filter(|server| !server.ip.is_empty())
            .map(|server| Server {
                ip: server.ip.clone(),
                port: server.port,
            }),
        width: Some(launcher.width),
        height: Some(launcher.height),
        fullscreen: Some(launcher.fullscreen),
        extra_jvm_args: Some(launcher.extra_jvm_args.clone()),
        extra_mc_args: Some(launcher.extra_mc_args.clone()),
        is_demo: Some(launcher.is_demo),
        ignore_invalid_minecraft_certificates: Some(launcher.ignore_invalid_minecraft_certificates),
        ignore_patch_discrepancies: Some(launcher.ignore_patch_discrepancies),
        extra_class_paths: Some(launcher.extra_class_paths.clone()),
        gc: Some(launcher.gc.clone()),
        launcher_name: Some(launcher.launcher_name.clone()),
        wrap_command: Some(launcher.wrap_command.clone()),
        execute_before_launch: Some(launcher.execute_before_launch.clone()),
        execute_after_launch: Some(launcher.execute_after_launch.clone()),
        skip_check_files: Some(launcher.skip_check_files),
        quit_app_after_launch: Some(launcher.quit_app_after_launch),
    }
}

/// The instance's background picture, or an empty image when it has none.
///
/// Slint can only read a runtime path through Rust, so the file is decoded here —
/// and it is decoded rather than handed to `Image::load_from_path` because the
/// file the instance keeps has no extension (`add_background_image` copies it to
/// a plain `background`) and because Slint's `image-default-formats` covers png
/// and jpeg only, while the app's own `image` dependency also reads webp and gif
/// — the same decode the window background and the content overlays use.
///
/// An image that could not be decoded reports a zero width, which is the same
/// test the Vue makes with the `<img>`'s `load` and `error` events.
fn load_background(instance: &Instance) -> Image {
    if !instance.has_background {
        return Image::default();
    }
    let path = slint_instance::get_background_path(&instance.id);
    let decode = || {
        let bytes = std::fs::read(&path).ok()?;
        let rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some((width, height, rgba.into_raw()))
    };
    match decode() {
        Some((width, height, rgba)) => Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
            rgba.as_slice(),
            width,
            height,
        )),
        None => {
            log::warn!("failed to decode '{}'", path.display());
            Image::default()
        }
    }
}

/// The mod loader's display name, or empty for a vanilla instance — which is
/// what both the card and the loader select read as "Vanilla".
fn loader_name(instance: &Instance) -> String {
    instance
        .config
        .runtime
        .mod_loader_type
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default()
}

/// Fills the delete dialog's card and opens it (the Vue's
/// `confirmDeleteInstance.instanceToDelete = …; visible = true`).
fn open_delete_dialog(ui: &App, instance: &Instance) {
    let delete = ui.global::<DeleteInstanceState>();
    delete.set_id(instance.id.clone().into());
    delete.set_name(instance.config.name.clone().into());
    delete.set_loader(loader_name(instance).into());
    delete.set_minecraft_version(instance.config.runtime.minecraft.clone().into());
    delete.set_has_background(instance.has_background);
    delete.set_background(load_background(instance));
    // `calculatePlaytime` on the instance being deleted, which the dialog's
    // `watch(..., { immediate: true })` resolves before it can show anything.
    let playtime = slint_instance::calculate_playtime(&instance.id).unwrap_or_default();
    let (kind, value) = format_play_time(playtime);
    delete.set_playtime_kind(kind.into());
    delete.set_playtime_value(value.into());
    delete.set_deleting(false);
    ui.global::<Dialogs>()
        .set_confirm_delete_instance_visible(true);
}

/// Registers the delete dialog's two callbacks.
fn setup_delete_dialog(ui: &App) {
    let delete = ui.global::<DeleteInstanceState>();

    {
        let weak = ui.as_weak();
        delete.on_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Dialogs>()
                    .set_confirm_delete_instance_visible(false);
            }
        });
    }

    {
        let weak = ui.as_weak();
        delete.on_confirm(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            if ui.global::<DeleteInstanceState>().get_deleting() {
                return;
            }
            let id = ui.global::<DeleteInstanceState>().get_id().to_string();
            ui.global::<DeleteInstanceState>().set_deleting(true);

            // Removing the instance directory walks the whole tree, so it belongs
            // on the blocking pool rather than on the UI thread — the original
            // runs it in a Tauri command, off the UI thread too.
            let weak = weak.clone();
            crate::runtime::spawn_blocking(move || {
                let result = slint_instance::delete_instance(&id);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<DeleteInstanceState>().set_deleting(false);
                    match result {
                        Ok(()) => {
                            // The Vue's `loadInstances()` +
                            // `ensureCurrentInstanceAvailable()`, which the game
                            // view's own reload does: it keeps the selection when
                            // it is still there and falls back to the first.
                            ui.global::<Dialogs>()
                                .set_confirm_delete_instance_visible(false);
                            ui.global::<GameState>().invoke_refresh();
                        }
                        Err(error) => log::error!("failed to delete the instance: {error}"),
                    }
                });
            });
        });
    }
}
