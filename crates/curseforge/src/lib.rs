// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The CurseForge API client.
//!
//! The request plumbing this is built on: the mirror-first
//! `request_with_fallback` with its `response_is_valid` test and its API-key
//! gate, the `apply_query` coercion (strings verbatim, everything else through
//! `Value::to_string`, `null` dropped), and `compute_fingerprint` with its
//! private MurmurHash2.
//!
//! `build.rs` bakes in `CURSEFORGE_API_KEY`, which decides whether the
//! official-API fallback exists.

pub mod error;

use error::*;
use serde_json::Value;
use shared::{HTTP_CLIENT, UrlExt};
use std::path::Path;
use url::Url;

// MCIM mirror of the CurseForge API. Does not require an API key.
// See https://github.com/mcmod-info-mirror/mcim-rust-api
const CACHE_BASE_URL: &str = "https://mod.mcimirror.top/curseforge";
// Official CurseForge API. Requires an API key. Only queried when the cache
// returns an empty or invalid result.
const OFFICIAL_BASE_URL: &str = "https://api.curseforge.com";
// MCIM translate API for mod summaries.
// See https://github.com/mcmod-info-mirror/translate-mod-summary
const TRANSLATE_BASE_URL: &str = "https://mod.mcimirror.top/translate";

// CurseForge API key baked in at build time by `build.rs` from the
// `CURSEFORGE_API_KEY` environment variable. Empty when the variable was unset.
const API_KEY: &str = env!("CURSEFORGE_API_KEY");

/// Minecraft's game id in the CurseForge API.
pub const MINECRAFT_GAME_ID: i64 = 432;

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

/// Fetch the translated summaries of the given CurseForge mods. Mods without a
/// translation are simply absent from the response.
pub async fn get_mod_translations(mod_ids: &[i64]) -> Result<Value> {
    let body = serde_json::json!({ "modids": mod_ids });
    send_request(
        TRANSLATE_BASE_URL,
        &reqwest::Method::POST,
        &["curseforge"],
        Some(&body),
        None,
    )
    .await
}

/// Fingerprint lookup. Returns the raw API response; its `data.exactMatches`
/// lists the mods whose files carry one of the given fingerprints.
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
    let (chunks, remainder) = data.as_chunks::<4>();
    for chunk in chunks {
        let mut k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        hash = hash.wrapping_mul(M);
        hash ^= k;
    }
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

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use serde_json::json;

    #[test]
    fn murmur2_matches_the_reference_values() {
        // The standard MurmurHash2 (32-bit, little-endian) with seed 1.
        assert_eq!(murmur2(b"", 1), 0x5bd1_5e36);
        assert_eq!(murmur2(b"abc", 1), 0x60a4_fcc1);
        assert_eq!(murmur2(b"hello", 1), 0xa631_918e);
    }

    #[test]
    fn build_url_joins_the_segments_below_the_base() {
        let url = build_url("https://api.curseforge.com", &["v1", "mods", "search"]).unwrap();
        assert_eq!(url.as_str(), "https://api.curseforge.com/v1/mods/search");
    }

    #[test]
    fn build_url_rejects_a_non_url_base() {
        assert!(build_url("not a url", &["v1"]).is_err());
    }

    #[test]
    fn a_response_is_valid_only_when_its_data_is_non_empty() {
        assert!(!response_is_valid(&json!({})));
        assert!(!response_is_valid(&json!({ "data": null })));
        assert!(!response_is_valid(&json!({ "data": "" })));
        assert!(!response_is_valid(&json!({ "data": [] })));
        assert!(!response_is_valid(&json!({ "data": {} })));

        assert!(response_is_valid(&json!({ "data": "x" })));
        assert!(response_is_valid(&json!({ "data": [1] })));
        assert!(response_is_valid(&json!({ "data": { "a": 1 } })));
        assert!(response_is_valid(&json!({ "data": 0 })));
    }

    #[test]
    fn apply_query_drops_nulls_and_stringifies_the_rest() {
        let url = Url::parse("https://example.com/").unwrap();
        let builder = apply_query(
            HTTP_CLIENT.get(url),
            &json!({ "text": "hello", "null": null, "size": 8, "flag": true }),
        );
        let request = builder.build().expect("a valid request");
        let pairs: Vec<(String, String)> = request
            .url()
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();

        assert!(pairs.contains(&("text".to_string(), "hello".to_string())));
        assert!(pairs.contains(&("size".to_string(), "8".to_string())));
        assert!(pairs.contains(&("flag".to_string(), "true".to_string())));
        assert!(!pairs.iter().any(|(key, _)| key == "null"));
    }

    fn temp_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time moves forward")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "conic-fingerprint-{}-{nanos}-{counter}",
            std::process::id()
        ))
    }

    #[test]
    fn a_fingerprint_ignores_whitespace_bytes() {
        let path = temp_path();
        std::fs::write(&path, b"a b\tc\nd\r").expect("writable temp file");
        // Stripped, the bytes are "abcd", whose MurmurHash2 is the reference
        // value below.
        assert_eq!(compute_fingerprint(&path).unwrap(), 0xc93f_7a16);
        std::fs::remove_file(&path).expect("clean up the temp file");
    }
}
