// Amethyst Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

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
