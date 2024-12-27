// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

pub mod install;
pub mod version_list;

const DEFAULT_META_URL: &str = "https://download.mcbbs.net/optifine";

// todo: 支持optifine安装选项

pub struct InstallOptifineOptions {
    /// Use "optifine.OptiFineForgeTweaker" instead of "optifine.OptiFineTweaker" for tweakClass.
    ///
    ///If you want to install upon forge, you should use this.
    pub use_forge_tweaker: Option<bool>,

    /// When you want to install a version over another one.
    ///
    /// Like, you want to install liteloader over a forge version. You should fill this with that forge version id.
    pub inherits_from: Option<String>,

    /// Override the newly installed version id.
    ///
    /// If this is absent, the installed version id will be either generated or provided by installer.
    pub version_id: Option<String>,

    /// The remote url of the Optifine installer.
    pub remote: Option<String>,
}
