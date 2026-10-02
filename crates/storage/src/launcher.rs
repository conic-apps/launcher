// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher's own data directory.

use std::path::{Path, PathBuf};

use platform::{OsFamily, PLATFORM_INFO};

/// Everything the launcher keeps for itself, apart from the shared Minecraft
/// installation and the instances (see [`crate::MinecraftLocation`] and
/// [`crate::InstancesLocation`]).
#[derive(Debug, Clone)]
pub struct LauncherLocation {
    pub root: PathBuf,
    pub accounts: PathBuf,
    /// The `authlib-injector.jar` the Yggdrasil flow launches with.
    pub authlib_injector: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub music: PathBuf,
    /// The Conic Nexus dynamic library and any other native helper.
    pub native: PathBuf,
    /// The per-run scratch directory, under the OS temp directory rather than
    /// the root: it is deleted on exit and must never be confused with data.
    pub temp: PathBuf,
    /// The settings file. It is inside this location, which is why the location
    /// itself is chosen by the bootstrap file and not by a setting.
    pub config: PathBuf,
}

impl LauncherLocation {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let temp = std::env::temp_dir().join(format!("conic-launcher-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).expect("Could not create temp dir");
        Self {
            accounts: root.join("accounts"),
            authlib_injector: root.join("authlib-injector.jar"),
            cache: match PLATFORM_INFO.os_family {
                OsFamily::Windows => root.join("cache"),
                _ => std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".cache/conic-launcher"))
                    .unwrap_or_else(|| root.join("cache")),
            },
            logs: root.join("logs"),
            music: root.join("music"),
            native: root.join("native"),
            temp,
            config: root.join("config.toml"),
            root,
        }
    }

    /// Creates the directories the launcher opens before writing to them.
    ///
    /// `logs` is created here rather than by the logger: `Settings → About`
    /// opens the folder whether or not anything has been logged yet, and the
    /// logger may not run at all.
    pub fn init(&self) {
        std::fs::create_dir_all(&self.music).expect("Unable to create application data directory");
        let _ = std::fs::create_dir_all(&self.logs);
        let _ = std::fs::create_dir_all(&self.native);
    }
}
