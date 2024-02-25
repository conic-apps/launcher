/*
 * Amethyst Launcher Core
 * Copyright (C) 2023 Broken-Deer <old_driver__@outlook.com> and contributors
 *
 * This program is free software, you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

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
