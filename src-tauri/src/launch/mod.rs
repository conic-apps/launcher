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

//! A launcher for game
//!
//! This module contains the [`Launcher`] [`LaunchOptions`] struct, the `launch` function for
//! launching a game, and several error types that may result from
//! working with [`Launcher`].
//!
//! # Examples
//!
//! There are multiple ways to create a new [`LaunchOptions`] from a string literal:
//!
//! ```
//! use aml_core::core::folder::MinecraftLocation;
//! use aml_core::launch::options::LaunchOptions;
//!
//! async fn fn_name() {
//!     let version_id = "1.19.4";
//!     let minecraft = MinecraftLocation::new(".minecraft");
//!     let options = LaunchOptions::new("1.19.4", &minecraft);
//! }
//! ```
//!
//! Then you can modify it with user custom options,
//! The step of creating default startup options is omitted here, in order to test through us here
//! assuming that the default options have been passed as parameters:
//!
//! ```
//! use aml_core::launch::options::LaunchOptions;
//!
//!  async fn fn_name2(default_options: &LaunchOptions) {
//!     let mut options = default_options.clone();
//!     options.game_profile.name = "Broken Deer".to_string();
//! }
//! ```
//!
//! Finally, you can build a [`Launcher`] instance, then launch the
//! game using [`Launcher::launch()`].
//!
//! ```
//! use aml_core::core::folder::MinecraftLocation;
//! use aml_core::core::JavaExec;
//! use aml_core::launch::launch::Launcher;
//! use aml_core::launch::options::LaunchOptions;
//! use crate::aml_core::launch::launch::Launch;
//!
//!  async fn fn_name3(options: LaunchOptions) {
//!     let mut launcher = Launcher::from_options(options, JavaExec::new("/path/to/java-home").await);
//!     launcher.launch(None, None, None, None).await.unwrap();
//! }
//! ```

pub mod argument;
pub mod options;
