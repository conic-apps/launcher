// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher's on-disk layout.
//!
//! Three independent locations make up an install, and [`LOCATIONS`] resolves
//! them once at startup:
//!
//! * [`LauncherLocation`] — the launcher's own data: `config.toml`, accounts,
//!   logs, music, the cache, the Conic Nexus native library and the per-run
//!   scratch directory.
//! * [`MinecraftLocation`] — the shared Minecraft installation: `assets`,
//!   `libraries`, `versions` and the launcher-managed Java `runtime`.
//! * [`InstancesLocation`] — the per-instance game directories.
//!
//! By default all three hang off one root: `instances` and `minecraft` are
//! subdirectories of it and the launcher data is the root itself. The root is
//! chosen from the platform and the packaging (see [`default_root`]); each of
//! the three can be redirected independently by the user.
//!
//! That redirection cannot live in `config.toml`, because the config file is
//! itself inside the launcher location: the choice has to be readable before the
//! config is. It lives in a small bootstrap file at a fixed platform anchor
//! instead — see [`bootstrap_path`] — and the launcher is restarted after it
//! changes.

mod bootstrap;
mod instances;
mod launcher;
mod minecraft;
mod platform;

use std::path::PathBuf;

use once_cell::sync::Lazy;

pub use bootstrap::{LocationOverrides, bootstrap_path, load_overrides, save_overrides};
pub use instances::InstancesLocation;
pub use launcher::LauncherLocation;
pub use minecraft::MinecraftLocation;
pub use platform::{APP_DIR_NAME, default_root, platform_anchor, platform_base};

/// The resolved layout, initialised once on first use.
///
/// Resolution reads the bootstrap file and probes the platform, so it has to
/// run before the logger (which writes into [`LauncherLocation::logs`]) and
/// before the config is loaded (which reads [`LauncherLocation::config`]).
pub static LOCATIONS: Lazy<Locations> = Lazy::new(Locations::resolve);

/// The three roots the launcher writes to.
#[derive(Debug, Clone)]
pub struct Locations {
    pub launcher: LauncherLocation,
    pub minecraft: MinecraftLocation,
    pub instances: InstancesLocation,
}

impl Locations {
    /// Resolves the layout from the platform default and the bootstrap file.
    pub fn resolve() -> Self {
        Self::from_overrides(default_root(), load_overrides())
    }

    /// Builds the layout from an explicit `root` and bootstrap overrides.
    ///
    /// A location with no override falls back to the root-derived default,
    /// which is what makes this testable without touching the environment.
    pub fn from_overrides(root: PathBuf, overrides: LocationOverrides) -> Self {
        let launcher_root = overrides.launcher.unwrap_or_else(|| root.clone());
        let minecraft_root = overrides
            .minecraft
            .unwrap_or_else(|| root.join("minecraft"));
        let instances_root = overrides
            .instances
            .unwrap_or_else(|| root.join("instances"));
        Self {
            launcher: LauncherLocation::new(launcher_root),
            minecraft: MinecraftLocation::new(minecraft_root),
            instances: InstancesLocation::new(instances_root),
        }
    }

    /// Creates the directories an install needs and seeds the files the game's
    /// installers expect.
    pub fn init(&self) {
        self.launcher.init();
        self.minecraft.init();
        self.instances.init();
    }
}
