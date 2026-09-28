// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/modrinth`: the Modrinth API client.
//!
//! The original is a Tauri plugin, so every function is wrapped in a
//! `#[command]` that exists only to hand the value back over IPC. There is no
//! plugin state and no `Channel` anywhere in it, so the mirror drops the whole
//! command layer and keeps the functions themselves — the two files can be
//! diffed against each other line for line apart from that.
//!
//! Everything the requests depend on is kept as it is: the same three base
//! URLs (the MCIM mirror for everything the mirror serves, the official API for
//! the one endpoint it does not), the same `shared` HTTP client and URL
//! builder, and the same response handling, including `get_versions_from_hashes`
//! treating a client error as "no hash matched".
//!
//! Two deviations, both deliberate:
//!
//!   * `get_multiple_projects` is **not** mirrored — it is broken upstream. It
//!     hands a `&[&str]` to `RequestBuilder::query`, and reqwest serializes a
//!     top-level sequence through `serde_urlencoded`'s pair serializer, which
//!     rejects a bare string, so the request never carries a query string and
//!     the command always fails. `get_projects` — which the same file already
//!     has, and which `crates/content/src/mods/remote.rs` already uses — takes
//!     the same ids as one JSON parameter and works.
//!   * The two request structs' fields are `pub`. Upstream they are private
//!     because Tauri deserializes them from the IPC payload and nothing else
//!     constructs them; here the app builds them directly.

pub mod error;

use error::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slint_shared::{HTTP_CLIENT, UrlExt};
use std::collections::HashMap;
use url::Url;

// const BASE_URL: &str = "https://api.modrinth.com";
const BASE_URL: &str = "https://mod.mcimirror.top/modrinth";
const OFFICIAL_BASE_URL: &str = "https://api.modrinth.com";
// MCIM translate API for project descriptions.
// See https://github.com/mcmod-info-mirror/translate-mod-summary
const TRANSLATE_BASE_URL: &str = "https://mod.mcimirror.top/translate";

/// Fetch the translated descriptions of the given Modrinth projects. Projects
/// without a translation are simply absent from the response.
pub async fn get_project_translations(project_ids: &[String]) -> Result<Value> {
    let url = Url::parse(TRANSLATE_BASE_URL)?
        .append_path(["modrinth"])
        .expect("Internal error");
    let body = serde_json::json!({ "project_ids": project_ids });
    Ok(HTTP_CLIENT
        .post(url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?)
}

#[derive(Serialize, Deserialize)]
pub struct SearchParameters {
    pub query: Option<String>,
    pub facets: Option<String>,
    pub index: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
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

pub async fn get_all_dependencies(id: &str) -> Result<Value> {
    let url = Url::parse(BASE_URL)?
        .append_path(["v2", "project", id, "dependencies"])
        .expect("Internal error");
    Ok(HTTP_CLIENT.get(url).send().await?.json().await?)
}

#[derive(Serialize, Deserialize)]
pub struct ListProjectVersionsParams {
    pub loaders: Option<String>,
    pub game_versions: Option<String>,
    pub featured: Option<String>,
    pub include_changelog: Option<String>,
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
pub async fn get_team_members(team_id: &str) -> Result<Value> {
    let url = Url::parse(OFFICIAL_BASE_URL)?
        .append_path(["v2", "team", team_id, "members"])
        .expect("Internal error");
    Ok(HTTP_CLIENT.get(url).send().await?.json().await?)
}
