// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

pub mod error;

use error::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};

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
    Ok(HTTP_CLIENT
        .get("https://api.modrinth.com/v2/search")
        .query(params)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_project(id_or_slug: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get(format!("https://api.modrinth.com/v2/project/{id_or_slug}"))
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_multiple_projects(ids: &[&str]) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get("https://api.modrinth.com/v2/projects")
        .query(ids)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_all_dependencies(id: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get(format!(
            "https://api.modrinth.com/v2/project/{id}/dependencies"
        ))
        .send()
        .await?
        .json()
        .await?)
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
    Ok(HTTP_CLIENT
        .get(format!(
            "https://api.modrinth.com/v2/project/{id_or_slug}/version"
        ))
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
