// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A disk cache for the images and other binary resources the UI fetches over
//! HTTP: project icons, galleries, README images.
//!
//! `Cache-Control` is obeyed where given; otherwise an `ETag` or `Last-Modified`
//! drives a conditional request. Only `http`/`https` URLs outside the user's own
//! network are fetched.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{debug, warn};
use once_cell::sync::{Lazy, OnceCell};
use reqwest::header::{CACHE_CONTROL, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use url::Url;

use crate::HTTP_CLIENT;

/// The shared client has no timeout of its own; without this a silent host would
/// hold a card's icon open indefinitely.
const TIMEOUT: Duration = Duration::from_secs(15);

/// Bodies larger than this are served but not stored.
const MAX_ITEM_BYTES: usize = 16 * 1024 * 1024;

const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;

/// Pruning to exactly [`MAX_TOTAL_BYTES`] would re-run on the next write.
const PRUNE_TO: u64 = MAX_TOTAL_BYTES / 5 * 4;

/// Until the app sets a directory the cache is inert and every [`fetch`] is a
/// plain request.
static CACHE_DIR: OnceCell<PathBuf> = OnceCell::new();

/// Points the cache at a directory. Settled once, like
/// [`crate::set_system_proxy`], before the first request.
pub fn set_dir(dir: PathBuf) {
    if let Err(error) = std::fs::create_dir_all(&dir) {
        warn!("Failed to create the HTTP cache directory {dir:?}: {error}");
        return;
    }
    let _ = CACHE_DIR.set(dir);
}

/// The bytes of `url`, from the cache when fresh and from the network otherwise.
///
/// `None` for a URL this cache will not serve or a request that failed; callers
/// draw their fallback. A `data:` URL is the caller's to decode.
pub async fn fetch(url: &str) -> Option<Vec<u8>> {
    get(url, false).await
}

/// [`fetch`], ignoring the cache and asking the server.
pub async fn force_fetch(url: &str) -> Option<Vec<u8>> {
    get(url, true).await
}

async fn get(url: &str, reload: bool) -> Option<Vec<u8>> {
    if url.is_empty() || url.starts_with("data:") {
        return None;
    }
    let path = entry_path(url);
    let cached = match &path {
        Some(path) => read_entry(path).await,
        None => None,
    };

    if !reload
        && let Some(entry) = &cached
        && entry.is_fresh(now())
    {
        // The one line that tells a disk hit from a network fetch — the first
        // question about a slow or stale panel.
        debug!(
            "Cache hit for {url}: {} bytes, stored {} second(s) ago",
            entry.bytes.len(),
            now().saturating_sub(entry.stored_at)
        );
        return Some(entry.bytes.clone());
    }
    if cached.is_some() {
        debug!("{url} is cached but stale; asking the server again");
    }

    if !is_reachable(url).await {
        return None;
    }

    let mut request = HTTP_CLIENT.get(url).timeout(TIMEOUT);
    if !reload && let Some(entry) = &cached {
        request = add_validators(request, entry);
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            warn!("Could not fetch {url}: {error}");
            return stale(cached);
        }
    };

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        // Keep the body but refresh the clock, or a server rotating its ETag
        // would make every later fetch ask again.
        let Some(entry) = cached else {
            warn!("{url} answered 304 with nothing cached");
            return None;
        };
        let entry = Entry {
            stored_at: now(),
            ..entry
        };
        debug!("{url} was revalidated: 304, the cached body is still current");
        store(url, path, &entry).await;
        return Some(entry.bytes);
    }

    let status = response.status();
    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                warn!("{url} answered 429; treating it as a failure and serving the stale copy");
            } else {
                warn!("{url} answered {status}: {error}");
            }
            return stale(cached);
        }
    };
    debug!(
        "Fetched {url}: {status}, {} bytes",
        response.content_length().unwrap_or(0)
    );
    let headers = response.headers().clone();
    let Ok(body) = response.bytes().await else {
        warn!("{url} answered {status} but its body could not be read");
        return stale(cached);
    };
    let entry = Entry {
        max_age: max_age_of(&headers),
        etag: header_of(&headers, &ETAG),
        last_modified: header_of(&headers, &LAST_MODIFIED),
        stored_at: now(),
        bytes: body.to_vec(),
    };
    store(url, path, &entry).await;
    Some(entry.bytes)
}

/// A stale icon beats no icon when a request fails.
fn stale(cached: Option<Entry>) -> Option<Vec<u8>> {
    cached.map(|entry| entry.bytes)
}

/// Writes an entry and keeps the directory inside its limit.
async fn store(url: &str, path: Option<PathBuf>, entry: &Entry) {
    let Some(path) = path else {
        return;
    };
    if entry.bytes.len() > MAX_ITEM_BYTES {
        debug!(
            "Not caching {url}: {} bytes is over the {MAX_ITEM_BYTES} byte per-item limit",
            entry.bytes.len()
        );
        return;
    }
    if let Err(error) = write_entry(&path, entry).await {
        warn!("Failed to cache {url}: {error}");
        return;
    }
    prune_if_over().await;
}

fn add_validators(request: reqwest::RequestBuilder, entry: &Entry) -> reqwest::RequestBuilder {
    let request = match &entry.etag {
        Some(etag) => request.header(IF_NONE_MATCH, etag),
        None => request,
    };
    match &entry.last_modified {
        Some(last_modified) => request.header(IF_MODIFIED_SINCE, last_modified),
        None => request,
    }
}

/// One cached response. Validators are kept verbatim: reformatting an opaque
/// `ETag` would turn a `304` into a resend.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    /// `Cache-Control: max-age=…`, in seconds. `None` means revalidate rather
    /// than serve without asking.
    max_age: Option<u64>,
    etag: Option<String>,
    last_modified: Option<String>,
    /// Unix seconds.
    stored_at: u64,
    #[serde(skip)]
    bytes: Vec<u8>,
}

impl Entry {
    fn is_fresh(&self, now: u64) -> bool {
        let Some(max_age) = self.max_age else {
            return false;
        };
        // A backwards clock must make the entry "ask", never look brand new.
        now >= self.stored_at && now - self.stored_at < max_age
    }
}

/// A non-text validator cannot be sent back, so it is dropped rather than made
/// lossy.
fn header_of(
    headers: &reqwest::header::HeaderMap,
    name: &reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

/// The `max-age` of a `Cache-Control` header; every other directive is ignored,
/// so `no-store` arrives as no `max-age` and the entry is revalidated.
fn max_age_of(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(CACHE_CONTROL)?
        .to_str()
        .ok()?
        .split(',')
        .find_map(|directive| {
            let (name, value) = directive.split_once('=')?;
            name.trim()
                .eq_ignore_ascii_case("max-age")
                .then(|| value.trim().parse().ok())?
        })
}

fn entry_path(url: &str) -> Option<PathBuf> {
    let dir = CACHE_DIR.get()?;
    // The URL is hashed because case-sensitive path components such as
    // Modrinth's base62 id would collide on case-insensitive filesystems.
    Some(dir.join(format!("{:x}", Sha512::digest(url.as_bytes()))))
}

async fn read_entry(path: &Path) -> Option<Entry> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            debug!(
                "Could not read the cache entry at {}: {error}",
                path.display()
            );
            return None;
        }
    };
    let meta_len = match bytes.get(..4).and_then(|len| len.try_into().ok()) {
        Some(len) => u32::from_le_bytes(len) as usize,
        None => {
            warn!(
                "The cache entry at {} is truncated ({} bytes); it will be refetched",
                path.display(),
                bytes.len()
            );
            return None;
        }
    };
    let Some(meta_end) = 4usize.checked_add(meta_len) else {
        warn!(
            "The cache entry at {} declares an impossible metadata length; it will be \
             refetched",
            path.display()
        );
        return None;
    };
    let meta: Entry = match bytes.get(4..meta_end).map(serde_json::from_slice) {
        Some(Ok(meta)) => meta,
        _ => {
            warn!(
                "The cache entry at {} has unreadable metadata; it will be refetched",
                path.display()
            );
            return None;
        }
    };
    let Some(body) = bytes.get(meta_end..) else {
        warn!(
            "The cache entry at {} is missing its body; it will be refetched",
            path.display()
        );
        return None;
    };
    Some(Entry {
        bytes: body.to_vec(),
        ..meta
    })
}

async fn write_entry(path: &Path, entry: &Entry) -> std::io::Result<()> {
    let meta = serde_json::to_vec(entry).map_err(std::io::Error::other)?;
    let mut bytes = Vec::with_capacity(4 + meta.len() + entry.bytes.len());
    bytes.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&meta);
    bytes.extend_from_slice(&entry.bytes);
    tokio::fs::write(path, bytes).await?;
    note_written(entry.bytes.len() as u64);
    Ok(())
}

/// The directory size this process believes it has, counted in memory; drift
/// only matters until the next prune re-measures it.
struct Usage {
    bytes: Option<u64>,
}

/// Only held for arithmetic, never across an `.await`.
static USAGE: Lazy<Mutex<Usage>> = Lazy::new(|| Mutex::new(Usage { bytes: None }));

/// Cuts the directory back to [`PRUNE_TO`] once it is over [`MAX_TOTAL_BYTES`].
async fn prune_if_over() {
    let Some(dir) = CACHE_DIR.get() else {
        return;
    };
    {
        let Ok(usage) = USAGE.lock() else {
            return;
        };
        match usage.bytes {
            Some(bytes) if bytes > MAX_TOTAL_BYTES => {}
            _ => return,
        }
    }

    let mut entries = Vec::new();
    let mut reader = match tokio::fs::read_dir(dir).await {
        Ok(reader) => reader,
        Err(error) => {
            warn!("Failed to read the HTTP cache directory {dir:?}: {error}");
            return;
        }
    };
    // A read error ends the walk and makes the total an under-estimate, so the
    // warning below is what keeps that from looking like a small cache.
    loop {
        match reader.next_entry().await {
            Ok(Some(entry)) => {
                let Ok(metadata) = entry.metadata().await else {
                    continue;
                };
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                    .map(|since| since.as_secs())
                    .unwrap_or(0);
                entries.push((modified, metadata.len(), entry.path()));
            }
            Ok(None) => break,
            Err(error) => {
                warn!(
                    "Stopped reading the HTTP cache directory {dir:?} early: {error}; its \\
                     measured size is an under-estimate"
                );
                break;
            }
        }
    }
    entries.sort_unstable();
    debug!(
        "The HTTP cache is over {MAX_TOTAL_BYTES} bytes across {} entries; pruning",
        entries.len()
    );

    let mut total: u64 = entries.iter().map(|(_, size, _)| *size).sum();
    for (_, size, path) in &entries {
        if total <= PRUNE_TO {
            break;
        }
        match tokio::fs::remove_file(path).await {
            Ok(()) => total = total.saturating_sub(*size),
            Err(error) => debug!("Failed to prune {path:?}: {error}"),
        }
    }

    if let Ok(mut usage) = USAGE.lock() {
        *usage = Usage { bytes: Some(total) };
    }
}

fn note_written(size: u64) {
    let Ok(mut usage) = USAGE.lock() else {
        return;
    };
    match &mut usage.bytes {
        Some(bytes) => *bytes = bytes.saturating_add(size),
        None => *usage = Usage { bytes: Some(size) },
    }
}

/// Whether the launcher may send this URL. A private address would let a
/// publisher reach the user's machine, so every address a name resolves to is
/// checked.
async fn is_reachable(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        debug!("Refusing {url}: not a URL");
        return false;
    };
    if !scheme_allows(&parsed) {
        // Security refusals stay at `warn` so a release log can show them.
        warn!("Refusing {url}: only http and https are fetched");
        return false;
    }
    let Some(host) = parsed.host_str() else {
        warn!("Refusing {url}: it names no host to resolve");
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    match host.parse::<IpAddr>() {
        Ok(address) => is_public(address),
        Err(_) => match tokio::net::lookup_host((host, 0u16)).await {
            Ok(addresses) => {
                for address in addresses {
                    if !is_public(address.ip()) {
                        debug!("Refusing {url}: {host} resolves to {}", address.ip());
                        return false;
                    }
                }
                true
            }
            // A name that resolves to nothing will fail on its own; refusing it
            // here would lose the cached copy.
            Err(error) => {
                debug!("Failed to resolve {host}: {error}");
                true
            }
        },
    }
}

fn scheme_allows(parsed: &Url) -> bool {
    matches!(parsed.scheme(), "http" | "https")
}

fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_v4(address),
        IpAddr::V6(address) => is_public_v6(address),
    }
}

fn is_public_v4(address: Ipv4Addr) -> bool {
    !(address.is_loopback()
        || address.is_private()
        // Carrier-grade NAT, 100.64.0.0/10, which `is_private` does not cover
        // and which is just as much "the local network".
        || (address.octets()[0] == 100 && (64..128).contains(&address.octets()[1]))
        // Link-local, which is where 169.254.169.254 — the cloud metadata
        // address — lives.
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_multicast()
        || address.is_broadcast())
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    !(address.is_loopback()
        // Unique local, fc00::/7.
        || (address.segments()[0] & 0xfe00) == 0xfc00
        // Link-local, fe80::/10.
        || (address.segments()[0] & 0xffc0) == 0xfe80
        || address.is_unspecified()
        || address.is_multicast())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allows(url: &str) -> bool {
        Url::parse(url)
            .map(|parsed| scheme_allows(&parsed))
            .unwrap_or(false)
    }

    #[test]
    fn a_loopback_or_private_address_is_not_out_on_the_internet() {
        for address in [
            "127.0.0.1",
            "127.1.2.3",
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "::1",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "::",
            "ff02::1",
        ] {
            assert!(
                !is_public(address.parse().expect("an address")),
                "{address} should not be treated as public"
            );
        }
    }

    #[test]
    fn an_address_on_the_internet_is() {
        for address in [
            "1.1.1.1",
            "8.8.8.8",
            // Just outside each of the private ranges, so a mask that is one
            // bit too wide is caught.
            "9.255.255.255",
            "11.0.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.167.255.255",
            "192.169.0.0",
            "100.63.255.255",
            "100.128.0.0",
            "169.253.255.255",
            "2606:4700:4700::1111",
            "2001:db8::1",
            "fbff::1",
            "fe00::1",
        ] {
            assert!(
                is_public(address.parse().expect("an address")),
                "{address} should be treated as public"
            );
        }
    }

    #[test]
    fn only_http_and_https_are_fetched() {
        assert!(allows("https://cdn.example.com/a.png"));
        assert!(allows("http://cdn.example.com/a.png"));
        assert!(!allows("ftp://example.com/a.png"));
        assert!(!allows("file:///etc/passwd"));
        assert!(!allows("data:image/png;base64,AAAA"));
        assert!(!allows("not a url at all"));
    }

    fn cache_control(value: &str) -> Option<u64> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(CACHE_CONTROL, value.parse().expect("a header value"));
        max_age_of(&headers)
    }

    #[test]
    fn a_max_age_is_read_out_of_a_crowded_cache_control() {
        assert_eq!(cache_control("public, max-age=2678400"), Some(2678400));
        assert_eq!(cache_control("max-age=3600"), Some(3600));
        assert_eq!(cache_control("public,max-age=60,immutable"), Some(60));
        assert_eq!(cache_control("MAX-AGE=120"), Some(120));
    }

    #[test]
    fn a_response_without_a_max_age_has_no_freshness() {
        // The mirror sends an ETag and nothing else, and that has to mean "ask
        // again" rather than "keep forever" or "never keep".
        assert_eq!(cache_control("public"), None);
        assert_eq!(cache_control(""), None);
        assert_eq!(cache_control("no-store"), None);
        assert_eq!(cache_control("max-age=abc"), None);
        assert_eq!(max_age_of(&reqwest::header::HeaderMap::new()), None);
    }

    fn entry(max_age: Option<u64>, stored_at: u64) -> Entry {
        Entry {
            max_age,
            etag: Some("\"abc\"".to_string()),
            last_modified: None,
            stored_at,
            bytes: Vec::new(),
        }
    }

    #[test]
    fn an_entry_is_fresh_only_while_it_is_within_its_max_age() {
        let entry = entry(Some(60), 1_000);
        assert!(entry.is_fresh(1_000));
        assert!(entry.is_fresh(1_059));
        assert!(!entry.is_fresh(1_060));
        // A clock that went backwards must not make an entry look young.
        assert!(!entry.is_fresh(0));
    }

    #[test]
    fn an_entry_with_no_max_age_is_never_served_without_asking() {
        assert!(!entry(None, 1_000).is_fresh(1_000));
    }

    fn with_temp_dir<T>(name: &str, body: impl FnOnce(&Path) -> T) -> T {
        let dir = std::env::temp_dir().join(format!("conic-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let out = body(&dir);
        std::fs::remove_dir_all(&dir).ok();
        out
    }

    fn current_thread() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime")
    }

    #[test]
    fn the_metadata_prefix_and_the_body_survive_a_round_trip() {
        with_temp_dir("http-cache-round-trip", |dir| {
            let path = dir.join("entry");
            let entry = Entry {
                max_age: Some(3600),
                etag: Some("\"e-tag\"".to_string()),
                last_modified: Some("Sun, 08 Sep 2024 04:24:39 GMT".to_string()),
                stored_at: 1_700_000_000,
                // A zero byte and a 0xff: proof that nothing re-encoded the body
                // on the way through the format.
                bytes: vec![0x89, 0x50, 0x4e, 0x47, 0x00, 0xff],
            };
            let runtime = current_thread();
            runtime
                .block_on(write_entry(&path, &entry))
                .expect("written");

            let read = runtime.block_on(read_entry(&path)).expect("read back");
            assert_eq!(read.max_age, Some(3600));
            assert_eq!(read.etag.as_deref(), Some("\"e-tag\""));
            assert_eq!(
                read.last_modified.as_deref(),
                Some("Sun, 08 Sep 2024 04:24:39 GMT")
            );
            assert_eq!(read.stored_at, 1_700_000_000);
            assert_eq!(read.bytes, vec![0x89, 0x50, 0x4e, 0x47, 0x00, 0xff]);
        });
    }

    #[test]
    fn a_missing_or_truncated_entry_reads_as_nothing() {
        with_temp_dir("http-cache-truncated", |dir| {
            let runtime = current_thread();
            assert!(runtime.block_on(read_entry(&dir.join("missing"))).is_none());

            // Cut off inside the metadata: the prefix promises bytes that are
            // not there.
            let truncated = dir.join("truncated");
            std::fs::write(&truncated, [8u8, 0, 0, 0, b'{']).expect("written");
            assert!(runtime.block_on(read_entry(&truncated)).is_none());

            // A length that runs past the end of the file.
            let lying = dir.join("lying");
            std::fs::write(&lying, [0xffu8, 0xff, 0xff, 0xff]).expect("written");
            assert!(runtime.block_on(read_entry(&lying)).is_none());

            // Metadata that parses but leaves no body behind.
            let empty = dir.join("empty");
            let meta = br#"{"stored_at":0}"#;
            let mut bytes = (meta.len() as u32).to_le_bytes().to_vec();
            bytes.extend_from_slice(meta);
            std::fs::write(&empty, bytes).expect("written");
            assert_eq!(
                runtime
                    .block_on(read_entry(&empty))
                    .map(|entry| entry.bytes),
                Some(Vec::new())
            );
        });
    }

    #[test]
    fn the_same_url_always_names_the_same_entry_and_a_different_one_does_not() {
        // `entry_path` needs `CACHE_DIR` set, so the naming is checked against
        // the hash it is derived from.
        let key = |url: &str| format!("{:x}", Sha512::digest(url.as_bytes()));
        assert_eq!(key("https://a/b"), key("https://a/b"));
        assert_ne!(key("https://a/b"), key("https://a/c"));
        // Case is significant in a Modrinth path and has to stay significant in
        // the key, which is the whole reason the URL is not the filename.
        assert_ne!(
            key("https://cdn/data/k8iIgzXE"),
            key("https://cdn/data/k8iigzxe")
        );
    }
}
