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

use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    download::download_files, folder::MinecraftLocation, utils::unzip::Entry, version::Version,
    PLATFORM_INFO,
};

use super::vanilla::generate_libraries_downloads;

pub mod install;
pub mod install_profile;
pub mod legacy_install;
pub mod new_install;
pub mod version_list;

pub struct ForgeVersion {
    pub installer: ForgeVersionInstaller,
    pub universal: ForgeVersionUniversal,

    /// The Minecraft version
    pub mcversion: String,

    /// The forge Version
    pub version: String,
    pub r#type: ForgeVersionType,
}

pub struct ForgeVersionInstaller {
    pub md5: String,
    pub sha1: String,

    /// The url path to concat with forge maven
    pub path: String,
}

pub struct ForgeVersionUniversal {
    pub md5: String,
    pub sha1: String,

    /// The url path to concat with forge maven
    pub path: String,
}

pub enum ForgeVersionType {
    Buggy(String),
    Recommended(String),
    Common(String),
    Latest(String),
}

/// All the useful entries in forge installer jar
pub struct ForgeInstallerEntries {
    /// maven/net/minecraftforge/forge/${forgeVersion}/forge-${forgeVersion}-universal.jar
    pub forge_jar: Option<Entry>,

    /// maven/net/minecraftforge/forge/${forgeVersion}/forge-${forgeVersion}-universal.jar
    pub forge_universal_jar: Option<Entry>,

    /// data/client.lzma
    pub client_lzma: Option<Entry>,

    /// data/server.lzma
    pub server_lzma: Option<Entry>,
    /// install_profile.json
    pub install_profile_json: Option<Entry>,

    /// version.json
    pub version_json: Option<Entry>,

    /// forge-${forgeVersion}-universal.jar
    pub legacy_universal_jar: Option<Entry>,

    /// data/run.sh
    pub run_sh: Option<Entry>,

    /// data/run.bat
    pub run_bat: Option<Entry>,

    /// data/unix_args.txt
    pub unix_args: Option<Entry>,

    /// data/user_jvm_args.txt
    pub user_jvm_args: Option<Entry>,

    /// data/win_args.txt
    pub win_args: Option<Entry>,
}

pub struct ForgeInstallerEntriesPatten {
    /// maven/net/minecraftforge/forge/${forgeVersion}/forge-${forgeVersion}-universal.jar
    pub forge_jar: Option<Entry>,

    /// maven/net/minecraftforge/forge/${forgeVersion}/forge-${forgeVersion}-universal.jar
    pub forge_universal_jar: Option<Entry>,

    /// data/client.lzma
    pub client_lzma: Option<Entry>,

    /// data/server.lzma
    pub server_lzma: Option<Entry>,
    /// install_profile.json
    pub install_profile_json: Entry,

    /// version.json
    pub version_json: Entry,

    /// forge-${forgeVersion}-universal.jar
    pub legacy_universal_jar: Option<Entry>,

    /// data/run.sh
    pub run_sh: Option<Entry>,

    /// data/run.bat
    pub run_bat: Option<Entry>,

    /// data/unix_args.txt
    pub unix_args: Option<Entry>,

    /// data/user_jvm_args.txt
    pub user_jvm_args: Option<Entry>,

    /// data/win_args.txt
    pub win_args: Option<Entry>,
}

pub struct ForgeLegacyInstallerEntriesPatten {
    /// install_profile.json
    pub install_profile_json: Entry,

    /// forge-${forgeVersion}-universal.jar
    pub legacy_universal_jar: Entry,
}
pub struct RequiredVersion {
    pub installer: Option<RequiredVersionInstaller>,
    pub mcversion: String,
    pub version: String,
}

pub struct RequiredVersionInstaller {
    pub sha1: Option<String>,
    /// The url path to concat with forge maven
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstallForgeOptions {
    /// The alterative maven host to download library. It will try to use these host from the `[0]` to the `[maven.length - 1]`
    pub maven_host: Option<Vec<String>>,

    /// When you want to install a version over another one.
    ///
    /// Like, you want to install liteloader over a forge version.
    /// You should fill this with that forge version id.
    pub inherits_from: Option<String>,

    /// Override the newly installed version id.
    ///
    /// If this is absent, the installed version id will be either generated or provided by installer.
    pub version_id: Option<String>,

    /// New forge (>=1.13) require java to install. Can be a executor or java executable path.
    pub java: Option<PathBuf>,
}

pub enum ForgeType {
    New,
    Legacy,
    Bad,
}

pub async fn install(
    version: RequiredVersion,
    minecraft: MinecraftLocation,
    options: InstallForgeOptions,
) -> anyhow::Result<()> {
    install::install_forge(version, minecraft.clone(), Some(options.clone())).await?;
    let version = Version::from_versions_folder(minecraft.clone(), &options.version_id.unwrap())?;
    let resolved_version = version
        .parse(&minecraft, PLATFORM_INFO.get().unwrap())
        .await?;
    let downloads = generate_libraries_downloads(&resolved_version.libraries, &minecraft);
    download_files(downloads, false, false).await;
    Ok(())
}
