// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

pub mod error;

use error::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::{HTTP_CLIENT, UrlExt};
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

// const BASE_URL: &str = "api.modrinth.com";
const BASE_URL: &str = "mod.mcimirror.top/modrinth";

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

#[derive(Deserialize)]
pub struct ModFile {
    pub url: String,
    pub file_name: String,
    pub sha512: String,
    pub size_bytes: u64,
}
