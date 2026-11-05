// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher's on-disk layout.
//!
//! Three independent locations, resolved once by [`LOCATIONS`]:
//! [`LauncherLocation`] (the launcher's own data), [`MinecraftLocation`] (the
//! shared install) and [`InstancesLocation`].
//!
//! By default all three hang off one root: `minecraft` and `instances` are
//! subdirectories and the launcher data is the root itself. Each can be
//! redirected independently. The override lives in a bootstrap file at a fixed
//! platform anchor, not in `config.toml`, because it has to be readable before
//! the config.

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

/// The resolved layout. Initialised before the logger and the config, which
/// both read from it.
pub static LOCATIONS: Lazy<Locations> = Lazy::new(Locations::resolve);

#[derive(Debug, Clone)]
pub struct Locations {
    pub launcher: LauncherLocation,
    pub minecraft: MinecraftLocation,
    pub instances: InstancesLocation,
}

impl Locations {
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
        let locations = Self {
            launcher: LauncherLocation::new(launcher_root),
            minecraft: MinecraftLocation::new(minecraft_root),
            instances: InstancesLocation::new(instances_root),
        };
        // The only place all three paths are known together; a relocation is
        // confirmed from this line.
        log::info!(
            "launcher data at {}, minecraft at {}, instances at {}",
            locations.launcher.root.display(),
            locations.minecraft.root.display(),
            locations.instances.root.display()
        );
        locations
    }

    pub fn init(&self) {
        self.launcher.init();
        self.minecraft.init();
        self.instances.init();
    }
}
