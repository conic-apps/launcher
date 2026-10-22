// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Modrinth API client.
//!
//! The requests depend on three base URLs (the MCIM mirror for everything the
//! mirror serves, the official API when the mirror does not answer), the
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
//!     parameter, works and is what `crates/content/src/remote/mod.rs` uses.
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

fn build_url(base_url: &str, segments: &[&str]) -> Result<Url> {
    Ok(Url::parse(base_url)?.append_path(segments.iter().copied())?)
}

/// Sends `request` and decodes the answer, recording what happened along the way.
///
/// Every entry point in this crate is one request and one decode, and the ways
/// that can go wrong are otherwise indistinguishable from the caller: `send()`
/// and `.json()` both yield a `reqwest::Error` and neither carries the URL, and
/// a non-2xx is not an error at all unless somebody looks for it. A mirror that
/// answers `429` and a mirror that answers an HTML error page would reach the
/// caller as the same variant, so the status is recorded here.
///
/// `url` is passed separately because `RequestBuilder` cannot be asked what it is
/// about to send, and the URL is the one thing every one of these lines needs.
///
/// Any non-2xx is returned as an error rather than decoded. That is what lets
/// [`get_json_with_fallback`] ask the official API when the mirror answers with
/// an error page, instead of treating the page as a successful (but empty)
/// result.
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
    if let Err(error) = response.error_for_status_ref() {
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
        return Err(Error::Network(error));
    }
    response.json().await.map_err(|error| {
        warn!("{url} answered {status} but the body did not parse as JSON: {error}");
        Error::Network(error)
    })
}

/// Sends one request to `base_url` and decodes it.
async fn get_json_at<F>(base_url: &str, segments: &[&str], build: F) -> Result<Value>
where
    F: Fn(&Url) -> reqwest::RequestBuilder,
{
    let url = build_url(base_url, segments)?;
    get_json(&url, build(&url)).await
}

/// Queries the MCIM mirror first and the official API when the mirror fails.
///
/// The mirror is a cache, not the source of truth, so a transport error, a
/// non-2xx answer or an undecodable body is a reason to ask the official API
/// rather than to report the failure. A successful answer is used as-is, even
/// when it is empty: an empty result is a legitimate answer to a search.
async fn get_json_with_fallback<F>(segments: &[&str], build: F) -> Result<Value>
where
    F: Fn(&Url) -> reqwest::RequestBuilder,
{
    match get_json_at(BASE_URL, segments, &build).await {
        Ok(value) => Ok(value),
        Err(error) => {
            warn!("the Modrinth mirror did not answer ({error}); asking the official API");
            get_json_at(OFFICIAL_BASE_URL, segments, build).await
        }
    }
}

/// Fetch the translated descriptions of the given Modrinth projects. Projects
/// without a translation are simply absent from the response.
///
/// The translation service has no official counterpart, so this endpoint never
/// falls back.
pub async fn get_project_translations(project_ids: &[String]) -> Result<Value> {
    let body = serde_json::json!({ "project_ids": project_ids });
    get_json_at(TRANSLATE_BASE_URL, &["modrinth"], |url| {
        HTTP_CLIENT.post(url.clone()).json(&body)
    })
    .await
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
    let value = get_json_with_fallback(&["v2", "search"], |url| {
        HTTP_CLIENT.get(url.clone()).query(params)
    })
    .await?;
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
    get_json_with_fallback(&["v2", "project", id_or_slug], |url| {
        HTTP_CLIENT.get(url.clone())
    })
    .await
}

pub async fn get_all_dependencies(id: &str) -> Result<Value> {
    get_json_with_fallback(&["v2", "project", id, "dependencies"], |url| {
        HTTP_CLIENT.get(url.clone())
    })
    .await
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
    get_json_with_fallback(&["v2", "project", id_or_slug, "version"], |url| {
        HTTP_CLIENT.get(url.clone()).query(params)
    })
    .await
}

/// Look up the versions matching the given file hashes.
///
/// `algorithm` is the hash algorithm the API is asked for; this crate passes
/// `sha512`.
/// The response maps each requested hash to its version. Hashes that did not
/// match are simply absent; when none of the hashes match the API answers with
/// a client error, which is treated as an empty result here.
///
/// The mirror is asked first and the official API second, because the caller
/// writes the empty map into `content`'s identity cache with a 24-hour
/// lifetime: a mirror outage must not pin "these mods are unknown" for a day.
pub async fn get_versions_from_hashes(
    hashes: &[String],
    algorithm: &str,
) -> Result<HashMap<String, Value>> {
    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": algorithm,
    });
    let mirror_url = build_url(BASE_URL, &["v2", "version_files"])?;
    if let Some(matched) = fetch_versions_from_hashes(&mirror_url, &body, hashes.len()).await {
        info!(
            "Modrinth matched {}/{} of the requested hashes",
            matched.len(),
            hashes.len()
        );
        return Ok(matched);
    }
    warn!("the Modrinth mirror's hash lookup failed; asking the official API");
    let official_url = build_url(OFFICIAL_BASE_URL, &["v2", "version_files"])?;
    let matched = fetch_versions_from_hashes(&official_url, &body, hashes.len())
        .await
        .unwrap_or_default();
    info!(
        "Modrinth matched {}/{} of the requested hashes",
        matched.len(),
        hashes.len()
    );
    Ok(matched)
}

/// One attempt at the hash lookup, or `None` when the server did not answer
/// with a usable JSON object.
async fn fetch_versions_from_hashes(
    url: &Url,
    body: &Value,
    requested: usize,
) -> Option<HashMap<String, Value>> {
    let response = HTTP_CLIENT.post(url.clone()).json(body).send().await;
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            warn!("{url} could not be reached: {error}");
            return None;
        }
    };
    if let Err(error) = response.error_for_status_ref() {
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("<none>");
        warn!(
            "the Modrinth hash lookup for {requested} hash(es) at {url} failed ({error}); \
             a retry would have been due after {retry_after}"
        );
        return None;
    }
    match response.json().await {
        Ok(matched) => Some(matched),
        Err(error) => {
            warn!("{url} answered but the hash lookup did not parse as JSON: {error}");
            None
        }
    }
}

/// Fetch several projects in one request. `ids` accepts project ids or slugs.
pub async fn get_projects(ids: &[&str]) -> Result<Value> {
    let ids_param = serde_json::to_string(ids)?;
    get_json_with_fallback(&["v2", "projects"], |url| {
        HTTP_CLIENT
            .get(url.clone())
            .query(&[("ids", ids_param.clone())])
    })
    .await
}

/// Fetch the members of a project team. The mirror does not serve this
/// endpoint, so the official API is queried directly.
pub async fn get_team_members(team_id: &str) -> Result<Value> {
    get_json_at(
        OFFICIAL_BASE_URL,
        &["v2", "team", team_id, "members"],
        |url| HTTP_CLIENT.get(url.clone()),
    )
    .await
}
