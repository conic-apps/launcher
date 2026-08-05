// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use base64::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;

use crate::error::*;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YggdrasilServerInfo {
    /// See https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83#meta-%E4%B8%AD%E7%9A%84%E5%85%83%E6%95%B0%E6%8D%AE
    pub meta: HashMap<String, Value>,
    pub skin_domains: Vec<String>,
    pub signature_publickey: String,
}

pub async fn get_server_info(api_root: &str) -> Result<YggdrasilServerInfo> {
    Ok(HTTP_CLIENT.get(api_root).send().await?.json().await?)
}

pub async fn get_server_info_base64(api_root: &str) -> Result<String> {
    let api_response = HTTP_CLIENT.get(api_root).send().await?.bytes().await?;
    Ok(BASE64_STANDARD.encode(api_response))
}
