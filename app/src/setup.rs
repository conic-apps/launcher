// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The first-run setup wizard's "script".
//!
//! Three things need Rust. The import-instances screen fetches the Mojang
//! version manifest and writes an `instance.toml`; the Java settings screen asks
//! whether this machine can have a Java runtime downloaded for it at all, which
//! is a property of the platform rather than of the config; and the storage step
//! opens the folder picker and commits the location choice when the wizard
//! finishes or is skipped.
//!
//! Everything else the wizard touches is config (`AppConfig.language`,
//! `AppConfig.palette`, …) or account state the game view already owns
//! (`GameState.current-account-*`), so it needs no script of its own.
//!
//! # Why finishing can move data
//!
//! The storage step sits after the language step but the wizard keeps writing
//! to the *current* data directory (the config's `config.toml`, the account
//! files, the created instances) until it finishes. That is deliberate: the
//! launcher location itself lives in that config, so it cannot be applied
//! without a restart, and a restart in the middle of the wizard would lose every
//! later step. Instead `finish` writes the in-memory config — language and all —
//! into the chosen launcher directory and moves the rest of the data there, and
//! only then restarts.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use slint::{ComponentHandle, SharedString, Weak};

use crate::slint_backend::{App, AppConfig, GameState, SetupWizardState};
use instance::{InstanceConfig, InstanceLaunchConfig};

/// Which end of the Mojang manifest a wizard-created instance is built on —
/// `latest.release` or `latest.snapshot`.
#[derive(Clone, Copy)]
enum Channel {
    Release,
    Snapshot,
}

/// Registers the wizard's callbacks and answers the platform question its Java
/// screen asks.
pub fn setup(ui: &App, shared: Rc<RefCell<config::Config>>) {
    // The Mojang runtimes are published for x86-64 and arm64 on Windows and
    // macOS, and for x86-64 only on Linux.
    let platform = &platform::PLATFORM_INFO;
    let supported = match platform.os_family {
        platform::OsFamily::Windows | platform::OsFamily::Macos => matches!(
            platform.arch,
            platform::OsArch::X64 | platform::OsArch::Aarch64
        ),
        platform::OsFamily::Linux => platform.arch == platform::OsArch::X64,
    };
    ui.global::<SetupWizardState>()
        .set_jvm_auto_install_supported(supported);

    // Whether the storage step has already been answered. The view mounts it
    // only while this is false, so reopening the wizard does not ask again.
    ui.global::<SetupWizardState>()
        .set_storage_configured(storage::load_overrides().initialized);

    {
        let weak = ui.as_weak();
        ui.global::<SetupWizardState>()
            .on_create_latest_release(move || create_latest(&weak, Channel::Release));
    }
    {
        let weak = ui.as_weak();
        ui.global::<SetupWizardState>()
            .on_create_latest_snapshot(move || create_latest(&weak, Channel::Snapshot));
    }

    {
        // The storage rows only update themselves here; the choice is read back
        // and applied by `finish`.
        let weak = ui.as_weak();
        ui.global::<SetupWizardState>()
            .on_pick_storage(move |which| {
                let Some(ui) = weak.upgrade() else { return };
                pick_storage(&ui, which.as_str());
            });
    }
    {
        let shared = Rc::clone(&shared);
        let weak = ui.as_weak();
        ui.global::<SetupWizardState>().on_finish(move || {
            let Some(ui) = weak.upgrade() else { return };
            finish(&ui, &shared);
        });
    }
    {
        let shared = Rc::clone(&shared);
        let weak = ui.as_weak();
        ui.global::<SetupWizardState>().on_skip(move || {
            let Some(ui) = weak.upgrade() else { return };
            skip(&ui, &shared);
        });
    }
}

/// Opens the folder chooser for one storage row and shows the chosen path.
///
/// Nothing is written: the wizard commits the whole storage choice in `finish`,
/// because a launcher-data change moves `config.toml` and the process has to
/// restart into the new location.
fn pick_storage(ui: &App, which: &str) {
    let settings = ui.global::<AppConfig>();
    let current = match which {
        "launcher" => settings.get_launcher_location(),
        "minecraft" => settings.get_minecraft_location(),
        "instances" => settings.get_instances_location(),
        _ => return,
    };
    let Some(chosen) =
        crate::config_bridge::pick_directory("Select a folder", Path::new(current.as_str()))
    else {
        return;
    };
    let chosen = chosen.to_string_lossy().to_string().into();
    match which {
        "launcher" => settings.set_launcher_location(chosen),
        "minecraft" => settings.set_minecraft_location(chosen),
        "instances" => settings.set_instances_location(chosen),
        _ => {}
    }
}

/// Commits the storage choice and marks the wizard done.
///
/// This is where the location change is applied, and it is why the storage step
/// can sit right after the language step: the in-memory config (which already
/// holds the chosen language) is written into the new launcher directory, and
/// the data the rest of the wizard wrote — accounts most of all — is moved
/// there beside it. Only then does the process restart, so no step is lost.
fn finish(ui: &App, shared: &Rc<RefCell<config::Config>>) {
    let settings = ui.global::<AppConfig>();
    let mut overrides = collect_overrides(&settings);
    let changed = overrides.launcher.is_some()
        || overrides.minecraft.is_some()
        || overrides.instances.is_some();

    {
        let mut config = shared.borrow_mut();
        config.setup_completed = true;
    }

    // The config goes to the new launcher directory before the old one is moved,
    // so it is the copy with `setup_completed` (and the language) in it that
    // survives the restart; the move then skips the file that already exists.
    if let Some(new_launcher) = &overrides.launcher {
        let config = shared.borrow();
        if let Err(error) = config::save_config_to(&new_launcher.join("config.toml"), &config) {
            log::error!("failed to write the config to the new location: {error}");
        }
    }
    if let Some(new_launcher) = &overrides.launcher
        // `logs` may hold the file the logger has open, and `locations.toml` is
        // the bootstrap pointer itself — which sits inside the default launcher
        // root and must stay at the platform anchor, not move with the data.
        && let Err(error) = move_contents(
            &storage::LOCATIONS.launcher.root,
            new_launcher,
            &["logs", "locations.toml"],
        )
    {
        log::error!("failed to move the launcher data: {error}");
    }
    if let Some(new_minecraft) = &overrides.minecraft
        && let Err(error) = move_contents(&storage::LOCATIONS.minecraft.root, new_minecraft, &[])
    {
        log::error!("failed to move the Minecraft install: {error}");
    }
    if let Some(new_instances) = &overrides.instances
        && let Err(error) = move_contents(&storage::LOCATIONS.instances.root, new_instances, &[])
    {
        log::error!("failed to move the instances: {error}");
    }

    // Mark the storage step answered even when nothing moved, so reopening the
    // wizard does not ask about storage again.
    overrides.initialized = true;
    if let Err(error) = storage::save_overrides(&overrides) {
        log::error!("failed to save the storage locations: {error}");
    }

    if changed {
        // Only the roots that moved are recorded in the file; an unchanged one
        // stays absent.
        // The new roots only take effect on the next start.
        crate::config_bridge::relaunch();
    } else {
        let config = shared.borrow();
        if let Err(error) = config::save_config(&config) {
            log::error!("failed to save config: {error}");
        }
    }
}

/// Marks the wizard done without applying a pending choice: the user gets the
/// defaults (or whatever storage was already configured), and the picks made in
/// the skipped session are discarded because nothing was ever written for them.
fn skip(_ui: &App, shared: &Rc<RefCell<config::Config>>) {
    {
        let mut config = shared.borrow_mut();
        config.setup_completed = true;
    }
    // Mark the storage step answered. Skipping keeps the current/default roots,
    // but the wizard should not offer to change them again.
    let mut overrides = storage::load_overrides();
    overrides.initialized = true;
    if let Err(error) = storage::save_overrides(&overrides) {
        log::error!("failed to save the storage locations: {error}");
    }
    let config = shared.borrow();
    if let Err(error) = config::save_config(&config) {
        log::error!("failed to save config: {error}");
    }
    // TODO(notification-center): once the notification center exists, raise a
    // notification here — "you skipped the setup wizard, click to return".
}

/// The overrides the wizard's rows ask for: each row that differs from the
/// resolved root becomes an override, an unchanged one stays absent.
fn collect_overrides(settings: &AppConfig) -> storage::LocationOverrides {
    let mut overrides = storage::LocationOverrides::default();
    let launcher = PathBuf::from(settings.get_launcher_location().as_str());
    if launcher != storage::LOCATIONS.launcher.root {
        overrides.launcher = Some(launcher);
    }
    let minecraft = PathBuf::from(settings.get_minecraft_location().as_str());
    if minecraft != storage::LOCATIONS.minecraft.root {
        overrides.minecraft = Some(minecraft);
    }
    let instances = PathBuf::from(settings.get_instances_location().as_str());
    if instances != storage::LOCATIONS.instances.root {
        overrides.instances = Some(instances);
    }
    overrides
}

/// Moves every entry of `from` into `to`, leaving `skip_entries` behind.
///
/// A same-filesystem move is a `rename`; a cross-device one is a copy followed
/// by a delete. An entry the destination already has is left where it is rather
/// than overwritten — the config the caller just wrote is the case that matters.
fn move_contents(from: &Path, to: &Path, skip_entries: &[&str]) -> std::io::Result<()> {
    if from == to {
        return Ok(());
    }
    if to.starts_with(from) || from.starts_with(to) {
        // Moving a directory into itself, or into a child of itself, would
        // recurse. The picker usually prevents this; a warned no-op is enough
        // for the odd case it does not.
        log::warn!(
            "not moving nested locations ({} and {})",
            from.display(),
            to.display()
        );
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if skip_entries
            .iter()
            .any(|skipped| name.to_string_lossy() == *skipped)
        {
            continue;
        }
        let destination = to.join(&name);
        if destination.exists() {
            continue;
        }
        let source = entry.path();
        if std::fs::rename(&source, &destination).is_ok() {
            continue;
        }
        copy_recursively(&source, &destination)?;
        if source.is_dir() {
            std::fs::remove_dir_all(&source)?;
        } else {
            std::fs::remove_file(&source)?;
        }
    }
    Ok(())
}

fn copy_recursively(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursively(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
    }
    Ok(())
}

/// Fetches the manifest, then writes the instance.
///
/// The buttons are disabled while this runs and after it has failed, so it is
/// never reached twice for one channel. The flags are checked anyway rather than
/// assumed, since a second run would race the first one's write into the same
/// instance folder.
fn create_latest(weak: &Weak<App>, channel: Channel) {
    let Some(ui) = weak.upgrade() else { return };
    let state = ui.global::<SetupWizardState>();
    match channel {
        Channel::Release => {
            if state.get_creating_release() || state.get_release_error() {
                return;
            }
            state.set_creating_release(true);
        }
        Channel::Snapshot => {
            if state.get_creating_snapshot() || state.get_snapshot_error() {
                return;
            }
            state.set_creating_snapshot(true);
        }
    }
    // The name is read here rather than on the worker: the global is only
    // reachable from this thread.
    let name: SharedString = match channel {
        Channel::Release => state.get_latest_release_name(),
        Channel::Snapshot => state.get_latest_snapshot_name(),
    };

    let weak = weak.clone();
    crate::runtime::spawn(async move {
        let version = install::get_minecraft_version_list()
            .await
            .map(|manifest| match channel {
                Channel::Release => manifest.latest.release,
                Channel::Snapshot => manifest.latest.snapshot,
            })
            .map_err(|error| error.to_string());
        // The version, not the new instance's id, is what the button shows
        // afterwards.
        let created = match version {
            Ok(version) => create_instance(&name, &version)
                .await
                .map(|_| SharedString::from(version))
                .map_err(|error| error.to_string()),
            Err(error) => Err(error),
        };
        let _ = weak.clone().upgrade_in_event_loop(move |ui| {
            let state = ui.global::<SetupWizardState>();
            match (channel, &created) {
                (Channel::Release, Ok(version)) => {
                    state.set_created_release(version.clone());
                    state.set_release_error(false);
                }
                (Channel::Release, Err(error)) => {
                    log::error!("failed to create the latest-release instance: {error}");
                    state.set_release_error(true);
                }
                (Channel::Snapshot, Ok(version)) => {
                    state.set_created_snapshot(version.clone());
                    state.set_snapshot_error(false);
                }
                (Channel::Snapshot, Err(error)) => {
                    log::error!("failed to create the latest-snapshot instance: {error}");
                    state.set_snapshot_error(true);
                }
            }
            match channel {
                Channel::Release => state.set_creating_release(false),
                Channel::Snapshot => state.set_creating_snapshot(false),
            }
            // The instance list is read again when the game view is entered,
            // which is where the wizard's last step ends. Asking for the
            // refresh here rather than there is the same work one page
            // earlier, and it is what `create_instance.rs` does for the
            // create-instance dialog.
            ui.global::<GameState>().invoke_refresh();
        });
    });
}

/// Writes the instance with the launch config the wizard uses. No `id`, so the
/// instance gets a random one.
async fn create_instance(name: &str, version: &str) -> Result<String, instance::Error> {
    let mut config = InstanceConfig::new(name, version);
    config.launch_config = InstanceLaunchConfig {
        enable_instance_specific_settings: false,
        ..Default::default()
    };
    instance::create_instance(config, None).await
}
