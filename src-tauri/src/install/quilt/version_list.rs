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

use super::{QuiltArtifactVersion, QuiltVersion, DEFAULT_META_URL};

pub async fn get_quilt_version_list(remote: Option<String>) -> Result<Vec<QuiltArtifactVersion>> {
    let remote = match remote {
        None => DEFAULT_META_URL.to_string(),
        Some(remote) => remote,
    };
    let url = format!("{remote}/v3/versions/loader");
    let response = reqwest::get(url).await?;
    Ok(response.json().await?)
}

pub async fn get_quilt_version_list_from_mcversion(
    remote: Option<String>,
    mcversion: &str,
) -> Result<Vec<QuiltVersion>> {
    let remote = match remote {
        None => DEFAULT_META_URL.to_string(),
        Some(remote) => remote,
    };
    let url = format!("{remote}/v3/versions/loader/{mcversion}");
    let response = reqwest::get(url).await?;
    Ok(response.json().await?)
}
