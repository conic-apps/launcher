// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

pub mod error;

use error::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::{HTTP_CLIENT, UrlExt};
use std::collections::HashMap;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use url::Url;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("modrinth")
        .invoke_handler(tauri::generate_handler![
            cmd_search_projects,
            cmd_get_project,
            cmd_get_multiple_projects,
            cmd_get_all_dependencies,
            cmd_list_project_versions,
        ])
        .build()
}

#[command]
async fn cmd_search_projects(params: SearchParameters) -> Result<Value> {
    search_projects(&params).await
}

#[command]
async fn cmd_get_project(id_or_slug: &str) -> Result<Value> {
    get_project(id_or_slug).await
}

#[command]
async fn cmd_get_multiple_projects(ids: Vec<&str>) -> Result<Value> {
    get_multiple_projects(&ids).await
}

#[command]
async fn cmd_get_all_dependencies(id: &str) -> Result<Value> {
    get_all_dependencies(id).await
}

// const BASE_URL: &str = "https://api.modrinth.com";
const BASE_URL: &str = "https://mod.mcimirror.top/modrinth";
const OFFICIAL_BASE_URL: &str = "https://api.modrinth.com";

#[command]
async fn cmd_list_project_versions(
    id_or_slug: &str,
    params: ListProjectVersionsParams,
) -> Result<Value> {
    list_project_versions(id_or_slug, &params).await
}

#[derive(Serialize, Deserialize)]
pub struct SearchParameters {
    query: Option<String>,
    facets: Option<String>,
    index: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

pub async fn search_projects(params: &SearchParameters) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "search"])
        .expect("Internal error");
    Ok(HTTP_CLIENT
        .get(url)
        .query(params)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_project(id_or_slug: &str) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "project", id_or_slug])
        .expect("Internal error");
    Ok(HTTP_CLIENT.get(url).send().await?.json().await?)
}

pub async fn get_multiple_projects(ids: &[&str]) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "projects"])
        .expect("Internal error");
    Ok(HTTP_CLIENT.get(url).query(ids).send().await?.json().await?)
}

pub async fn get_all_dependencies(id: &str) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "project", id, "dependencies"])
        .expect("Internal error");
    Ok(HTTP_CLIENT.get(url).send().await?.json().await?)
}

#[derive(Serialize, Deserialize)]
pub struct ListProjectVersionsParams {
    loaders: Option<String>,
    game_versions: Option<String>,
    featured: Option<String>,
    include_changelog: Option<String>,
}

pub async fn list_project_versions(
    id_or_slug: &str,
    params: &ListProjectVersionsParams,
) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "project", id_or_slug, "version"])
        .expect("Internal error");
    Ok(HTTP_CLIENT
        .get(url)
        .query(params)
        .send()
        .await?
        .json()
        .await?)
}

/// Look up the versions matching the given file hashes.
///
/// `algorithm` accepts `sha1`, `sha512`, `sha256`, `md5` and `murmd5`.
/// The response maps each requested hash to its version. Hashes that did not
/// match are simply absent; when none of the hashes match the API answers with
/// a client error, which is treated as an empty result here.
pub async fn get_versions_from_hashes(
    hashes: &[String],
    algorithm: &str,
) -> Result<HashMap<String, Value>> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "version_files"])
        .expect("Internal error");
    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": algorithm,
    });
    let response = HTTP_CLIENT.post(url).json(&body).send().await?;
    if response.status().is_client_error() {
        return Ok(HashMap::new());
    }
    Ok(response.json().await?)
}

/// Fetch several projects in one request. `ids` accepts project ids or slugs.
pub async fn get_projects(ids: &[&str]) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "projects"])
        .expect("Internal error");
    let ids_param = serde_json::to_string(ids)?;
    Ok(HTTP_CLIENT
        .get(url)
        .query(&[("ids", ids_param)])
        .send()
        .await?
        .json()
        .await?)
}

/// Fetch the members of a project team. The mirror does not serve this
/// endpoint, so the official API is queried directly.
pub async fn get_project_members(team_id: &str) -> Result<Value> {
    let url = Url::parse(OFFICIAL_BASE_URL)?
        .append_path(["v2", "team", team_id, "members"])
        .expect("Internal error");
    Ok(HTTP_CLIENT.get(url).send().await?.json().await?)
}
