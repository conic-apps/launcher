// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

pub mod error;

use error::*;
use serde_json::Value;
use shared::{HTTP_CLIENT, UrlExt};
use std::path::Path;
use tauri::{
    Runtime, command,
    plugin::{Builder, TauriPlugin},
};
use url::Url;

// MCIM mirror of the CurseForge API. Does not require an API key.
// See https://github.com/mcmod-info-mirror/mcim-rust-api
const CACHE_BASE_URL: &str = "https://mod.mcimirror.top/curseforge";
// Official CurseForge API. Requires an API key. Only queried when the cache
// returns an empty or invalid result.
const OFFICIAL_BASE_URL: &str = "https://api.curseforge.com";

// CurseForge API key baked in at build time by `build.rs` from the
// `CURSEFORGE_API_KEY` environment variable. Empty when the variable was unset.
const API_KEY: &str = env!("CURSEFORGE_API_KEY");

/// Minecraft's game id in the CurseForge API.
pub const MINECRAFT_GAME_ID: i64 = 432;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("curseforge")
        .invoke_handler(tauri::generate_handler![
            cmd_search_mods,
            cmd_get_mod,
            cmd_get_mods,
            cmd_get_featured_mods,
            cmd_get_mod_description,
            cmd_get_mod_files,
            cmd_get_mod_file,
            cmd_get_files,
            cmd_get_mod_file_changelog,
            cmd_get_mod_file_download_url,
        ])
        .build()
}

#[command]
async fn cmd_search_mods(params: Value) -> Result<Value> {
    search_mods(&params).await
}

#[command]
async fn cmd_get_mod(mod_id: i64) -> Result<Value> {
    get_mod(mod_id).await
}

#[command]
async fn cmd_get_mods(body: Value) -> Result<Value> {
    get_mods(&body).await
}

#[command]
async fn cmd_get_featured_mods(body: Value) -> Result<Value> {
    get_featured_mods(&body).await
}

#[command]
async fn cmd_get_mod_description(mod_id: i64, params: Value) -> Result<Value> {
    get_mod_description(mod_id, &params).await
}

#[command]
async fn cmd_get_mod_files(mod_id: i64, params: Value) -> Result<Value> {
    get_mod_files(mod_id, &params).await
}

#[command]
async fn cmd_get_mod_file(mod_id: i64, file_id: i64) -> Result<Value> {
    get_mod_file(mod_id, file_id).await
}

#[command]
async fn cmd_get_files(body: Value) -> Result<Value> {
    get_files(&body).await
}

#[command]
async fn cmd_get_mod_file_changelog(mod_id: i64, file_id: i64) -> Result<Value> {
    get_mod_file_changelog(mod_id, file_id).await
}

#[command]
async fn cmd_get_mod_file_download_url(mod_id: i64, file_id: i64) -> Result<Value> {
    get_mod_file_download_url(mod_id, file_id).await
}

fn build_url(base_url: &str, segments: &[&str]) -> Result<Url> {
    Ok(Url::parse(base_url)?
        .append_path(segments.iter().copied())
        .expect("Internal error"))
}

fn apply_query(builder: reqwest::RequestBuilder, params: &Value) -> reqwest::RequestBuilder {
    let Some(object) = params.as_object() else {
        return builder;
    };
    let query: Vec<(String, String)> = object
        .iter()
        .filter_map(|(key, value)| match value {
            Value::Null => None,
            Value::String(value) => Some((key.clone(), value.clone())),
            value => Some((key.clone(), value.to_string())),
        })
        .collect();
    if query.is_empty() {
        builder
    } else {
        builder.query(&query)
    }
}

async fn send(builder: reqwest::RequestBuilder) -> Result<Value> {
    Ok(builder.send().await?.json().await?)
}

/// A response is considered valid when it carries a non-empty `data` field.
fn response_is_valid(value: &Value) -> bool {
    match value.get("data") {
        None => false,
        Some(Value::Null) => false,
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(values)) => !values.is_empty(),
        Some(Value::Object(values)) => !values.is_empty(),
        Some(Value::Bool(_) | Value::Number(_)) => true,
    }
}

async fn send_request(
    base_url: &str,
    method: &reqwest::Method,
    segments: &[&str],
    params: Option<&Value>,
    api_key: Option<&str>,
) -> Result<Value> {
    let url = build_url(base_url, segments)?;
    let mut builder = HTTP_CLIENT.request(method.clone(), url);
    if let Some(params) = params {
        builder = if *method == reqwest::Method::POST {
            builder.json(params)
        } else {
            apply_query(builder, params)
        };
    }
    if let Some(api_key) = api_key {
        builder = builder.header("x-api-key", api_key);
    }
    send(builder).await
}

/// Requests the cache (mirror) first, then falls back to the official API when
/// the cached result is empty or invalid. The fallback only happens when an API
/// key is configured; without one the cached result is returned as-is.
async fn request_with_fallback(
    method: &reqwest::Method,
    segments: &[&str],
    params: Option<&Value>,
) -> Result<Value> {
    let cache_result = send_request(CACHE_BASE_URL, method, segments, params, None).await;
    if matches!(&cache_result, Ok(value) if response_is_valid(value)) {
        return cache_result;
    }
    if API_KEY.is_empty() {
        return cache_result;
    }
    send_request(OFFICIAL_BASE_URL, method, segments, params, Some(API_KEY)).await
}

pub async fn search_mods(params: &Value) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &["v1", "mods", "search"],
        Some(params),
    )
    .await
}

pub async fn get_mod(mod_id: i64) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &["v1", "mods", &mod_id.to_string()],
        None,
    )
    .await
}

pub async fn get_mods(body: &Value) -> Result<Value> {
    request_with_fallback(&reqwest::Method::POST, &["v1", "mods"], Some(body)).await
}

pub async fn get_featured_mods(body: &Value) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::POST,
        &["v1", "mods", "featured"],
        Some(body),
    )
    .await
}

pub async fn get_mod_description(mod_id: i64, params: &Value) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &["v1", "mods", &mod_id.to_string(), "description"],
        Some(params),
    )
    .await
}

pub async fn get_mod_files(mod_id: i64, params: &Value) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &["v1", "mods", &mod_id.to_string(), "files"],
        Some(params),
    )
    .await
}

pub async fn get_mod_file(mod_id: i64, file_id: i64) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &[
            "v1",
            "mods",
            &mod_id.to_string(),
            "files",
            &file_id.to_string(),
        ],
        None,
    )
    .await
}

pub async fn get_files(body: &Value) -> Result<Value> {
    request_with_fallback(&reqwest::Method::POST, &["v1", "mods", "files"], Some(body)).await
}

pub async fn get_mod_file_changelog(mod_id: i64, file_id: i64) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &[
            "v1",
            "mods",
            &mod_id.to_string(),
            "files",
            &file_id.to_string(),
            "changelog",
        ],
        None,
    )
    .await
}

pub async fn get_mod_file_download_url(mod_id: i64, file_id: i64) -> Result<Value> {
    request_with_fallback(
        &reqwest::Method::GET,
        &[
            "v1",
            "mods",
            &mod_id.to_string(),
            "files",
            &file_id.to_string(),
            "download-url",
        ],
        None,
    )
    .await
}

/// Fingerprint lookup. Returns the mods whose files carry one of the given
/// fingerprints, keyed by the requested fingerprint.
pub async fn get_fingerprint_matches(game_id: i64, fingerprints: &[u32]) -> Result<Value> {
    let body = serde_json::json!({ "fingerprints": fingerprints });
    request_with_fallback(
        &reqwest::Method::POST,
        &["v1", "fingerprints", &game_id.to_string()],
        Some(&body),
    )
    .await
}

/// Compute the CurseForge fingerprint of a file: every ASCII whitespace byte
/// (`0x09`, `0x0A`, `0x0D`, `0x20`) is stripped, then MurmurHash2 with seed 1
/// is computed over the remaining bytes.
pub fn compute_fingerprint<P: AsRef<Path>>(path: P) -> Result<u32> {
    let bytes = std::fs::read(path)?;
    let normalized: Vec<u8> = bytes
        .into_iter()
        .filter(|byte| !matches!(byte, 0x09 | 0x0A | 0x0D | 0x20))
        .collect();
    Ok(murmur2(&normalized, 1))
}

/// MurmurHash2 (32-bit, little-endian reads).
fn murmur2(data: &[u8], seed: u32) -> u32 {
    const M: u32 = 0x5bd1e995;
    const R: u32 = 24;
    let mut hash = seed ^ data.len() as u32;
    let mut chunks = data.chunks_exact(4);
    for chunk in &mut chunks {
        let mut k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        hash = hash.wrapping_mul(M);
        hash ^= k;
    }
    let remainder = chunks.remainder();
    for (index, byte) in remainder.iter().enumerate() {
        hash ^= (*byte as u32) << (8 * index);
    }
    if !remainder.is_empty() {
        hash = hash.wrapping_mul(M);
    }
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(M);
    hash ^= hash >> 15;
    hash
}

#[tokio::test]
async fn test_fallback() {
    // Endpoints cached by the mirror return data with a `sync_at` field.
    let cached = search_mods(&serde_json::from_str::<Value>("{}").unwrap())
        .await
        .unwrap();
    dbg!(cached["sync_at"].as_str().is_some());
    // Endpoints not cached by the mirror (e.g. description) fall back to the
    // official API and carry no `sync_at` field.
    let description = get_mod_description(238222, &serde_json::from_str::<Value>("{}").unwrap())
        .await
        .unwrap();
    dbg!(description);
}
