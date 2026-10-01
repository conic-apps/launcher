// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The first-run setup wizard's "script" (`src/views/SetupView.vue` and the six
//! `views/setup/SetupWizard*.vue` screens).
//!
//! Two things need Rust. The import-instances screen fetches the Mojang version
//! manifest and writes an `instance.toml` — the `@conic/install` and
//! `@conic/instance` calls the Vue makes from that component — and the Java
//! settings screen asks whether this machine can have a Java runtime downloaded
//! for it at all, which is a property of the platform rather than of the config.
//!
//! Everything else the wizard touches is config (`AppConfig.language`,
//! `AppConfig.palette`, …) or account state the game view already owns
//! (`GameState.current-account-*`), so it needs no script of its own.

use slint::{ComponentHandle, SharedString, Weak};

use crate::slint_backend::{App, GameState, SetupWizardState};
use slint_instance::{InstanceConfig, InstanceLaunchConfig};

/// Which end of the Mojang manifest a wizard-created instance is built on —
/// `latest.release` or `latest.snapshot`.
#[derive(Clone, Copy)]
enum Channel {
    Release,
    Snapshot,
}

/// Registers the wizard's callbacks and answers the platform question its Java
/// screen asks.
pub fn setup(ui: &App) {
    // `SetupWizardJavaSettings.vue`'s `isSupportedJVMAutoInstallPlatform`: the
    // Mojang runtimes are published for x86-64 and arm64 on Windows and macOS,
    // and for x86-64 only on Linux. The Vue reads the same two fields off the
    // `window.__PLATFORM__` the Rust host fills at startup.
    let platform = &slint_platform::PLATFORM_INFO;
    let supported = match platform.os_family {
        slint_platform::OsFamily::Windows | slint_platform::OsFamily::Macos => matches!(
            platform.arch,
            slint_platform::OsArch::X64 | slint_platform::OsArch::Aarch64
        ),
        slint_platform::OsFamily::Linux => platform.arch == slint_platform::OsArch::X64,
    };
    ui.global::<SetupWizardState>()
        .set_jvm_auto_install_supported(supported);

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
}

/// `createLatestReleaseInstance()` / `createLatestSnapshotInstance()`: fetch the
/// manifest, then write the instance.
///
/// The Vue's buttons carry `pointer-events: none` while they are running and
/// after they have failed, so it never reaches this twice for one channel. The
/// flags are checked anyway rather than assumed, since a second run would race
/// the first one's write into the same instance folder.
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
    // The instance is named after the translated `setup.importInstances.
    // latestRelease` / `…latestSnapshot`, so the name is read here rather than
    // on the worker: the global is only reachable from this thread.
    let name: SharedString = match channel {
        Channel::Release => state.get_latest_release_name(),
        Channel::Snapshot => state.get_latest_snapshot_name(),
    };

    let weak = weak.clone();
    crate::runtime::spawn(async move {
        let version = slint_install::get_minecraft_version_list()
            .await
            .map(|manifest| match channel {
                Channel::Release => manifest.latest.release,
                Channel::Snapshot => manifest.latest.snapshot,
            })
            .map_err(|error| error.to_string());
        // The version, not the new instance's id, is what the button shows
        // afterwards — the Vue's `createdLatestRelease.value` is
        // `minecraftVersionManifest.latest.release`.
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
            // The Vue's `createInstance` writes the file and nothing else;
            // its instance list is read again when the game view is entered,
            // which is where the wizard's last step ends. Asking for the
            // refresh here rather than there is the same work one page
            // earlier, and it is what `create_instance.rs` does for the
            // create-instance dialog.
            ui.global::<GameState>().invoke_refresh();
        });
    });
}

/// `createInstance({ launch_config: { enable_instance_specific_settings:
/// false }, name, runtime: { minecraft } })` — the whole of the Vue's call. No
/// `id`, so the instance gets a random one, exactly as the original's.
async fn create_instance(name: &str, version: &str) -> Result<String, slint_instance::Error> {
    let mut config = InstanceConfig::new(name, version);
    config.launch_config = InstanceLaunchConfig {
        enable_instance_specific_settings: false,
        ..Default::default()
    };
    slint_instance::create_instance(config, None).await
}
