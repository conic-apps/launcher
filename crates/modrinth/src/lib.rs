// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Modrinth API client.
//!
//! The requests depend on three base URLs (the MCIM mirror for everything the
//! mirror serves, the official API for the one endpoint it does not), the
//! `shared` HTTP client and URL builder, and response handling in which
//! `get_versions_from_hashes` treats a client error as "no hash matched".
//!
//! Two deliberate choices shape the public surface:
//!
//!   * `get_multiple_projects` is omitted: it is broken upstream. It hands a
//!     `&[&str]` to `RequestBuilder::query`, and reqwest serializes a top-level
//!     sequence through `serde_urlencoded`'s pair serializer, which rejects a
//!     bare string, so the request never carries a query string and the call
//!     always fails. `get_projects`, which takes the same ids as one JSON
//!     parameter, works and is what `crates/content/src/mods/remote.rs` uses.
//!   * The two request structs' fields are `pub` so the app can build them
//!     directly.

pub mod error;

use error::*;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::{HTTP_CLIENT, UrlExt};
use std::collections::HashMap;
use url::Url;

const BASE_URL: &str = "https://mod.mcimirror.top/modrinth";
const OFFICIAL_BASE_URL: &str = "https://api.modrinth.com";
// MCIM translate API for project descriptions.
// See https://github.com/mcmod-info-mirror/translate-mod-summary
const TRANSLATE_BASE_URL: &str = "https://mod.mcimirror.top/translate";

/// Sends `request` and decodes the answer, recording what happened along the way.
///
/// Every entry point in this crate is one request and one decode, and the three
/// ways that can go wrong are otherwise indistinguishable from the caller:
/// `send()` and `.json()` both yield a `reqwest::Error`, neither carries the URL,
/// and a non-2xx is not an error at all unless somebody looks for it — a mirror
/// that answers `429` and a mirror that answers an HTML error page reach the
/// caller as the same variant. The status is the one fact that tells them apart,
/// so it is recorded here rather than reconstructed from a message.
///
/// `url` is passed separately because `RequestBuilder` cannot be asked what it is
/// about to send, and the URL is the one thing every one of these lines needs.
///
/// A successful decode is not logged. The callers that care about the *shape* of
/// the answer (a hit count, a match count) log that themselves.
async fn get_json(url: &Url, request: reqwest::RequestBuilder) -> Result<Value> {
    let url = url.as_str();
    let response = request.send().await.map_err(|error| {
        warn!("{url} could not be reached: {error}");
        Error::Network(error)
    })?;
    let status = response.status();
    if !status.is_success() {
        // `429` gets its own line because it is the one answer that says "come
        // back later" rather than "this is wrong" — and `content`'s mod cache
        // holds whatever this call returns for a day.
        match response.headers().get(reqwest::header::RETRY_AFTER) {
            Some(retry_after) => warn!(
                "{url} answered {status} and asked to retry after {}",
                retry_after
                    .to_str()
                    .unwrap_or("<an unreadable Retry-After>")
            ),
            None => warn!("{url} answered {status}"),
        }
    }
    response.json().await.map_err(|error| {
        warn!("{url} answered {status} but the body did not parse as JSON: {error}");
        Error::Network(error)
    })
}

/// Fetch the translated descriptions of the given Modrinth projects. Projects
/// without a translation are simply absent from the response.
pub async fn get_project_translations(project_ids: &[String]) -> Result<Value> {
    let url = Url::parse(TRANSLATE_BASE_URL)?.append_path(["modrinth"])?;
    let body = serde_json::json!({ "project_ids": project_ids });
    let request = HTTP_CLIENT.post(url.clone()).json(&body);
    get_json(&url, request).await
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
    let url = Url::parse(BASE_URL)?.append_path(["v2", "search"])?;
    let request = HTTP_CLIENT.get(url.clone()).query(params);
    let value = get_json(&url, request).await?;
    // The hit count is the only useful success signal a search has: an empty
    // result and a response that never carried a `hits` array look the same to
    // the caller, and the panel can only say "nothing found".
    if let Some(hits) = value.get("hits").and_then(Value::as_array) {
        info!(
            "Modrinth returned {} hit(s) for {:?}",
            hits.len(),
            params.query
        );
    } else {
        warn!("Modrinth's search answer carried no 'hits' array: {value}");
    }
    Ok(value)
}

pub async fn get_project(id_or_slug: &str) -> Result<Value> {
    let url = Url::parse(BASE_URL)?.append_path(["v2", "project", id_or_slug])?;
    let request = HTTP_CLIENT.get(url.clone());
    get_json(&url, request).await
}

pub async fn get_all_dependencies(id: &str) -> Result<Value> {
    let url = Url::parse(BASE_URL)?.append_path(["v2", "project", id, "dependencies"])?;
    let request = HTTP_CLIENT.get(url.clone());
    get_json(&url, request).await
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
    let url = Url::parse(BASE_URL)?.append_path(["v2", "project", id_or_slug, "version"])?;
    let request = HTTP_CLIENT.get(url.clone()).query(params);
    get_json(&url, request).await
}

/// Look up the versions matching the given file hashes.
///
/// `algorithm` is the hash algorithm the API is asked for; this crate passes
/// `sha512`.
/// The response maps each requested hash to its version. Hashes that did not
/// match are simply absent; when none of the hashes match the API answers with
/// a client error, which is treated as an empty result here.
pub async fn get_versions_from_hashes(
    hashes: &[String],
    algorithm: &str,
) -> Result<HashMap<String, Value>> {
    let url = Url::parse(BASE_URL)?.append_path(["v2", "version_files"])?;
    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": algorithm,
    });
    let response = HTTP_CLIENT.post(url.clone()).json(&body).send().await?;
    if let Err(error) = response.error_for_status_ref() {
        // This arm is the one that decides "no mod matches". The caller treats the
        // empty map as a definitive answer and `content` writes it into the mod
        // identity cache with a 24-hour lifetime, so a rate limit here does not
        // cost one lookup — it pins "these mods are unknown" for a day.
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("<none>");
        warn!(
            "the Modrinth hash lookup for {} hash(es) failed ({error}); treating them as \
             unmatched, and a retry would have been due after {retry_after}",
            hashes.len()
        );
        return Ok(HashMap::new());
    }
    let matched: HashMap<String, Value> = response.json().await?;
    info!(
        "Modrinth matched {}/{} of the requested hashes",
        matched.len(),
        hashes.len()
    );
    Ok(matched)
}

/// Fetch several projects in one request. `ids` accepts project ids or slugs.
pub async fn get_projects(ids: &[&str]) -> Result<Value> {
    let url = Url::parse(BASE_URL)?.append_path(["v2", "projects"])?;
    let ids_param = serde_json::to_string(ids)?;
    let request = HTTP_CLIENT.get(url.clone()).query(&[("ids", ids_param)]);
    get_json(&url, request).await
}

/// Fetch the members of a project team. The mirror does not serve this
/// endpoint, so the official API is queried directly.
pub async fn get_team_members(team_id: &str) -> Result<Value> {
    let url = Url::parse(OFFICIAL_BASE_URL)?.append_path(["v2", "team", team_id, "members"])?;
    let request = HTTP_CLIENT.get(url.clone());
    get_json(&url, request).await
}
