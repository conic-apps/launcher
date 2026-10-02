// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance's delete dialog and the shared config helpers it needs.

use super::*;

/// The instance's launch settings, seeded from the launcher's own.
///
/// Every field of the launcher's `config.launch` is named **except**
/// `skip_refresh_account`, which therefore goes back to the default. `java_path`
/// has no counterpart and is left empty.
pub(crate) fn inherit_launch_config(launcher: &LaunchConfig) -> InstanceLaunchConfig {
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
/// Slint reads a runtime path only through Rust, so the file is decoded here
/// with the app's own `image` dependency — the same `image::load_from_memory`
/// the window background and the content overlays use, which sniffs the format
/// from the bytes rather than the file name (the instance's copy is extensionless:
/// `add_background_image` writes it to a plain `background`).
///
/// An image that could not be decoded reports a zero width.
pub(crate) fn load_background(instance: &Instance) -> Image {
    if !instance.has_background {
        return Image::default();
    }
    let path = instance::get_background_path(&instance.id);
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
pub(crate) fn loader_name(instance: &Instance) -> String {
    instance
        .config
        .runtime
        .mod_loader_type
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default()
}

/// Fills the delete dialog's card and opens it.
pub(crate) fn open_delete_dialog(ui: &App, instance: &Instance) {
    let delete = ui.global::<DeleteInstanceState>();
    delete.set_id(instance.id.clone().into());
    delete.set_name(instance.config.name.clone().into());
    delete.set_loader(loader_name(instance).into());
    delete.set_minecraft_version(instance.config.runtime.minecraft.clone().into());
    delete.set_has_background(instance.has_background);
    delete.set_background(load_background(instance));
    // The playtime of the instance being deleted is resolved before the dialog
    // can show anything.
    let playtime = instance::calculate_playtime(&instance.id).unwrap_or_default();
    let (kind, value) = format_play_time(playtime);
    delete.set_playtime_kind(kind.into());
    delete.set_playtime_value(value.into());
    delete.set_deleting(false);
    ui.global::<Dialogs>()
        .set_confirm_delete_instance_visible(true);
}

/// Registers the delete dialog's two callbacks.
pub(crate) fn setup_delete_dialog(ui: &App) {
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
            // off the UI thread.
            let weak = weak.clone();
            crate::support::runtime::spawn(async move {
                let result = instance::delete_instance(&id).await;
                crate::ui::services::report::report(&weak, move |ui| {
                    ui.global::<DeleteInstanceState>().set_deleting(false);
                    match result {
                        Ok(()) => {
                            // The game view's own reload keeps the selection
                            // when it is still there and falls back to the
                            // first.
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
