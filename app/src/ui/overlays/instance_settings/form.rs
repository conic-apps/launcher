// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance settings form: opening it, reading it and writing it back.

use super::*;

/// Opens the overlay on the current instance, from the game view's settings
/// button.
///
/// The read of `instance.toml` runs on the runtime, and the fields are filled
/// in on the far side of it.
pub fn open(ui: &App) {
    let id = ui.global::<GameState>().get_current_id();
    if id.is_empty() {
        log::warn!("the instance settings were opened without a current instance");
        return;
    }
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let Some(instance) = instance::get_instance_by_id(id.as_str()).await else {
            log::warn!("the instance settings were opened without a current instance");
            return;
        };
        crate::ui::services::report::report(&weak, move |ui| show_instance(&ui, &instance));
    });
}

pub(crate) fn show_instance(ui: &App, instance: &Instance) {
    let state = ui.global::<InstanceSettingsState>();
    apply_instance(&state, instance);
    state.set_background(load_background(instance));
    state.set_visible(true);
    EDITING.with(|editing| *editing.borrow_mut() = Some(instance.clone()));
}

thread_local! {
    /// The instance the overlay is editing.
    ///
    /// `open` reads it off disk, on the runtime, and the callbacks that follow
    /// need its *stored* config synchronously — to diff an edit against what is
    /// on disk before the debounced write replaces it. That is the one instance
    /// the overlay covers, so it cannot change underneath: the game view behind
    /// it does not take a selection while an overlay is up.
    static EDITING: RefCell<Option<Instance>> = const { RefCell::new(None) };
}

pub(crate) fn current_instance() -> Option<Instance> {
    EDITING.with(|editing| editing.borrow().clone())
}

pub(crate) fn apply_instance(state: &InstanceSettingsState, instance: &Instance) {
    state.set_name(instance.config.name.clone().into());
    state.set_loader(loader_name(instance).into());
    state.set_minecraft_version(instance.config.runtime.minecraft.clone().into());
    state.set_has_background(instance.has_background);
    state.set_background_darkness(i32::from(instance.config.background_darkness));
    state.set_name_editing(false);
    apply_launch(state, &instance.config);
}

/// Puts an instance's launch settings into the overlay's fields.
///
/// Every number is a string because `TextInput` edits text (see
/// `globals/instance-settings.slint`); an `Option` the instance never set is an
/// empty field, which is where the placeholder shows.
pub(crate) fn apply_launch(state: &InstanceSettingsState, config: &InstanceConfig) {
    let launch = &config.launch_config;
    state.set_use_as_launcher_background(config.use_as_launcher_background);
    state.set_enable_specific(launch.enable_instance_specific_settings);
    state.set_width(number_text(launch.width).into());
    state.set_height(number_text(launch.height).into());
    state.set_fullscreen(launch.fullscreen.unwrap_or(false));
    state.set_quit_after_launch(launch.quit_app_after_launch.unwrap_or(false));
    state.set_skip_file_check(launch.skip_check_files.unwrap_or(false));
    state.set_auto_memory(launch.auto_memory.unwrap_or(false));
    state.set_max_memory(number_text(launch.max_memory).into());
    // An unset collector is an empty string rather than the default one, so no
    // option is highlighted and the "is this still the default?" test against
    // `"G1"` fails, which is what an unset `gc` should do.
    state.set_gc(match launch.gc {
        Some(ref gc) => crate::ui::services::app_config::gc_to_str(gc).into(),
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

pub(crate) fn number_text(value: Option<usize>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// A field's text as the number it names.
///
/// A blank field is `None`, which clears the setting rather than making it
/// zero. Anything else that does not parse keeps `previous`, the way
/// `crate::ui::services::app_config::parse_usize_keep` does for the settings screen: a field
/// mid-edit is briefly unparseable, and a save must not throw the setting away
/// over it.
pub(crate) fn parse_number(value: &str, previous: Option<usize>) -> Option<usize> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.parse().unwrap_or_else(|_| previous.unwrap_or(0)))
}

/// The instance's config with the overlay's edits merged into it.
///
/// Only what the overlay shows is read back: the name, `use_as_launcher_background`
/// and the launch settings are replaced, and the rest of the config — the icon,
/// the runtime and the group (which carries the starred flag) — is kept from the
/// instance the overlay opened with, so nothing the UI does not touch can be
/// lost through it.
pub(crate) fn collect(state: &InstanceSettingsState, mut config: InstanceConfig) -> InstanceConfig {
    config.name = state.get_name().to_string();
    config.use_as_launcher_background = state.get_use_as_launcher_background();
    // The slider is 0..=100, so the conversion only fails on a value the UI
    // cannot produce; keeping the stored one is the safe answer then, the way
    // the settings screen keeps an unparseable number.
    config.background_darkness =
        u8::try_from(state.get_background_darkness()).unwrap_or(config.background_darkness);
    let gc = state.get_gc();
    // Taken rather than borrowed, so the fields the overlay does not show can be
    // moved out of it and into the replacement below.
    let mut previous = std::mem::take(&mut config.launch_config);
    let launch = InstanceLaunchConfig {
        enable_instance_specific_settings: state.get_enable_specific(),
        // The whole `launch_config` is replaced on every write, which would
        // silently drop this field. The launcher reads it (`instance_java_path`),
        // so it is carried over here instead of cleared.
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
            Some(crate::ui::services::app_config::gc_from_str(gc.as_str()))
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
/// (the summary's title and its list card), `use_as_launcher_background`, the
/// flag the window background resolves on, and `background_darkness`, which the
/// window background reads through that resolution. Everything else the overlay
/// edits is read when the game is launched.
pub(crate) fn touches_game_view(before: &InstanceConfig, after: &InstanceConfig) -> bool {
    before.name != after.name
        || before.use_as_launcher_background != after.use_as_launcher_background
        || before.background_darkness != after.background_darkness
}

/// Schedules the write of an edited instance config.
///
/// The instance travels with the write rather than being looked up when it fires:
/// the overlay can be closed — and the current instance changed — in between.
/// A run of keystrokes is therefore one `instance.toml`.
pub(crate) fn schedule_save(
    save_timer: &Rc<Timer>,
    ui: &App,
    config: InstanceConfig,
    touches: bool,
) {
    let id = ui.global::<GameState>().get_current_id();
    if id.is_empty() {
        return;
    }
    let id = id.to_string();
    let weak = ui.as_weak();
    let save_timer = Rc::clone(save_timer);
    // The timer takes an `FnMut`, so the config it carries is taken out of an
    // `Option` rather than moved — and starting an armed timer replaces the
    // callback, which is what makes a second edit within the debounce replace
    // the first rather than queue behind it.
    let mut pending = Some(config);
    save_timer.start(TimerMode::SingleShot, SAVE_DEBOUNCE, move || {
        let Some(config) = pending.take() else {
            return;
        };
        let weak = weak.clone();
        let id = id.clone();
        crate::support::runtime::spawn(async move {
            if let Err(error) = instance::update_instance(config, &id).await {
                log::error!("failed to save the instance '{id}': {error}");
                return;
            }
            if !touches {
                return;
            }
            // The store is on disk, so the game view is asked to read it again
            // for the summary and the list card to re-render.
            crate::ui::services::report::report(&weak, move |ui| {
                ui.global::<GameState>().invoke_refresh()
            });
        });
    });
}

pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    setup_form(ui, config);
    setup_background(ui);
    setup_delete(ui);
}

pub(crate) fn setup_form(ui: &App, config: Rc<RefCell<config::Config>>) {
    let state = ui.global::<InstanceSettingsState>();
    let save_timer = Rc::new(Timer::default());

    {
        let weak = ui.as_weak();
        let save_timer = Rc::clone(&save_timer);
        state.on_changed(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance() else {
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
            let Some(instance) = current_instance() else {
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

    // The nine advanced fields back to what a fresh config holds, and nothing
    // else — the window size, the memory and the file check are left as they
    // are.
    {
        let weak = ui.as_weak();
        let save_timer = Rc::clone(&save_timer);
        state.on_reset_advanced(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(instance) = current_instance() else {
                return;
            };
            let defaults = config::Config::default().launch;
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
}

pub(crate) fn setup_background(ui: &App) {
    let state = ui.global::<InstanceSettingsState>();
    // The native picker, the copy into the instance, and a re-list so the
    // window background picks the new file up.
    {
        let weak = ui.as_weak();
        state.on_pick_background(move |filter_name| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let id = ui.global::<GameState>().get_current_id();
            if id.is_empty() {
                return;
            }
            // The picker is a native dialog, so it opens here on the UI thread;
            // the copy and the re-read behind it do not belong on it.
            let Some(path) =
                crate::ui::services::app_config::pick_image_file_named(filter_name.as_str())
            else {
                return;
            };
            let id = id.to_string();
            let weak = weak.clone();
            crate::support::runtime::spawn(async move {
                if let Err(error) = instance::add_background_image(&path, &id).await {
                    log::error!("failed to set the background of '{id}': {error}");
                    return;
                }
                // `has_background` is the file's existence, so the instance has
                // to be read again rather than patched.
                let Some(instance) = instance::get_instance_by_id(&id).await else {
                    return;
                };
                crate::ui::services::report::report(&weak, move |ui| {
                    let state = ui.global::<InstanceSettingsState>();
                    state.set_has_background(instance.has_background);
                    state.set_background(load_background(&instance));
                    // Picking a picture means wanting to see it, so it becomes
                    // the launcher background on the spot; the switch is left
                    // in place for turning it back off. This goes through
                    // `changed` so the write carries the rest of the form as it
                    // stands, and the reload underneath picks the file up.
                    state.set_use_as_launcher_background(true);
                    state.invoke_changed();
                    ui.global::<GameState>().invoke_refresh();
                });
            });
        });
    }

    {
        let weak = ui.as_weak();
        state.on_remove_background(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let id = ui.global::<GameState>().get_current_id();
            if id.is_empty() {
                return;
            }
            let id = id.to_string();
            let weak = weak.clone();
            crate::support::runtime::spawn(async move {
                if let Err(error) = instance::remove_background(&id).await {
                    log::error!("failed to remove the background of '{id}': {error}");
                    return;
                }
                crate::ui::services::report::report(&weak, move |ui| {
                    let state = ui.global::<InstanceSettingsState>();
                    state.set_has_background(false);
                    state.set_background(Image::default());
                    ui.global::<GameState>().invoke_refresh();
                });
            });
        });
    }
}

pub(crate) fn setup_delete(ui: &App) {
    let state = ui.global::<InstanceSettingsState>();
    {
        let weak = ui.as_weak();
        state.on_open_delete(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let id = ui.global::<GameState>().get_current_id();
            if id.is_empty() {
                return;
            }
            ui.global::<InstanceSettingsState>().set_visible(false);
            let id = id.to_string();
            let weak = weak.clone();
            crate::support::runtime::spawn(async move {
                let Some(instance) = instance::get_instance_by_id(&id).await else {
                    return;
                };
                crate::ui::services::report::report(&weak, move |ui| {
                    open_delete_dialog(&ui, &instance)
                });
            });
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
