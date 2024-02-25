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

use anyhow::Result;
use tokio::fs::{self, create_dir_all};

use crate::core::{folder::MinecraftLocation, version::LibraryInfo};

use super::{install_profile::InstallProfileLegacy, *};

pub(super) async fn install_legacy_forge_from_zip(
    entries: ForgeLegacyInstallerEntriesPatten,
    profile: InstallProfileLegacy,
    minecraft: MinecraftLocation,
    options: Option<InstallForgeOptions>,
) -> Result<()> {
    let options = match options {
        Some(options) => options,
        None => InstallForgeOptions {
            maven_host: None,
            libraries_download_concurrency: None,
            inherits_from: None,
            version_id: None,
            java: None,
        },
    };
    let mut version_json = profile.version_info.clone().unwrap();

    // apply override for inheritsFrom
    version_json.id = options.version_id.unwrap_or(version_json.id);
    version_json.inherits_from = match options.inherits_from {
        None => version_json.inherits_from,
        Some(inherits_from) => Some(inherits_from),
    };

    let root_path = minecraft.get_version_root(&version_json.id);
    let version_json_path = root_path.join(format!("{}.json", version_json.id));
    let install_profile_path = root_path.join("install_profile.json");
    create_dir_all(&version_json_path.parent().unwrap()).await?;
    let library = version_json.libraries.clone().unwrap();
    let library = library
        .iter()
        .find(|l| {
            l["name"]
                .as_str()
                .unwrap()
                .starts_with("net.minecraftforge:forge")
        })
        .unwrap();
    let library = LibraryInfo::from_value(library);

    fs::write(
        version_json_path,
        serde_json::to_string_pretty(&version_json)?,
    )
    .await?;
    fs::write(
        install_profile_path,
        serde_json::to_string_pretty(&profile)?,
    )
    .await?;

    create_dir_all(
        minecraft
            .get_library_by_path(&library.path)
            .parent()
            .unwrap(),
    )
    .await?;
    fs::write(
        minecraft.get_library_by_path(&library.path),
        entries.legacy_universal_jar.content,
    )
    .await?;

    Ok(())
}
