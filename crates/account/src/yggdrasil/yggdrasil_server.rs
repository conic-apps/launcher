use std::collections::HashMap;

use base64::prelude::*;
use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;
use url::Url;

use crate::error::*;

fn normalize_url(input: &str) -> Result<Url> {
    let s = input.trim();
    let s_lower = s.to_ascii_lowercase();
    let fixed = if s_lower.starts_with("http://") || s_lower.starts_with("https://") {
        s.to_string()
    } else if s.starts_with("//") {
        format!("https:{}", s)
    } else {
        format!("https://{}", s)
    };
    Ok(Url::parse(&fixed)?)
}

pub async fn add(api_root: &str) -> Result<()> {
    let normalized_api_root = normalize_url(api_root)?;
    let resolved_api_url = parse_ali(normalized_api_root.as_str()).await?;
    let mut servers = list_all().await?;
    servers.push(resolved_api_url);
    save_all(servers).await?;
    Ok(())
}

pub async fn delete(index_to_delete: usize) -> Result<()> {
    let servers = list_all().await?;
    let mut result = vec![];
    for (index, server) in servers.iter().enumerate() {
        if index == index_to_delete {
            continue;
        }
        result.push(server.to_string());
    }
    save_all(result).await?;
    Ok(())
}

async fn save_all(servers: Vec<String>) -> Result<()> {
    let yggdrasil_server_list_file = DATA_LOCATION.accounts.join("yggdrasil_servers.json");
    let serialized_server_list = serde_json::to_string_pretty(&servers)?;
    async_fs::write(yggdrasil_server_list_file, serialized_server_list).await?;
    Ok(())
}

pub async fn list_all() -> Result<Vec<String>> {
    let path = DATA_LOCATION.root.join("yggdrasil_servers.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = async_fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&data)?)
}

async fn parse_ali(url: &str) -> Result<String> {
    let response = HTTP_CLIENT.get(url).send().await?;
    let response_headers = response.headers();
    if let Some(api_location) = response_headers.get("x-authlib-injector-api-location") {
        return Ok(api_location
            .to_str()
            .map_err(|_| Error::InvalidALIResponse)?
            .to_string());
    };
    Ok(url.to_string())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YggdrasilServerInfo {
    /// See https://github.com/yushijinhun/authlib-injector/wiki/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83#meta-%E4%B8%AD%E7%9A%84%E5%85%83%E6%95%B0%E6%8D%AE
    pub meta: HashMap<String, Value>,
    pub skin_domains: Vec<String>,
    pub signature_public_key: String,
}

pub async fn get_server_info(api_root: &str) -> Result<YggdrasilServerInfo> {
    Ok(HTTP_CLIENT.get(api_root).send().await?.json().await?)
}

pub async fn get_server_info_base64(api_root: &str) -> Result<String> {
    let api_response = HTTP_CLIENT.get(api_root).send().await?.bytes().await?;
    Ok(BASE64_STANDARD.encode(api_response))
}
