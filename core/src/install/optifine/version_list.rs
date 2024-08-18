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
use serde::{Deserialize, Serialize};

use super::DEFAULT_META_URL;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptifineVersionListItem {
    pub _id: String,
    pub mcversion: String,
    pub patch: String,
    pub r#type: String,
    pub __v: i32,
    pub filename: String,
    pub forge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptifineVersionList(Vec<OptifineVersionListItem>);

impl OptifineVersionList {
    pub async fn new(mcversion: &str, remote: Option<String>) -> Result<Self> {
        let url = match remote {
            Some(remote) => format!("{remote}/{mcversion}"),
            None => format!("{DEFAULT_META_URL}/{mcversion}"),
        };
        Ok(reqwest::get(url)
            .await?
            .json::<OptifineVersionList>()
            .await?)
    }
}

#[tokio::test]
async fn test() {
    let list = OptifineVersionList::new("1.19.4", None).await;
    println!("{:#?}", list);
}
