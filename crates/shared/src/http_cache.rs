// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A disk cache for the images and other binary resources the UI fetches over
//! HTTP — project icons, galleries, the images a README references.
//!
//! It exists because the alternatives were both worse. Fetching every icon
//! again on each launch means a page of sixty costs sixty round-trips before
//! anything is drawn, and holding them in memory only means the *next* launch
//! pays it again. Caching the bytes on disk between the two is what the
//! in-memory `Image` memo in `content/icon.rs` cannot do.
//!
//! The callers are the content overlays' icon pipeline and the news overlay's
//! cover pipeline; anything that fetches a URL in order to show it goes through
//! here, so they share one format, one eviction policy and one set of limits.
//!
//! # What is stored
//!
//! One file per URL, named after the URL's SHA-512, holding the body **exactly
//! as it came off the wire** plus the few headers needed to ask whether it is
//! still current. Storing the wire bytes rather than a decoded bitmap is what
//! keeps the format independent of whatever decodes it later — a 67 kB JPEG is
//! 67 kB, where the decoded 72×72 RGBA the UI holds is a size this would have
//! to guess.
//!
//! The file is a `u32`, that many bytes of JSON metadata, then the body:
//!
//! ```text
//! ┌───────────────┬────────────────────┬────────────────────────────┐
//! │ u32 LE n      │ meta JSON (n bytes)│ body (the rest of the file)│
//! └───────────────┴────────────────────┴────────────────────────────┘
//! ```
//!
//! One file rather than a body and a sidecar because a crash between two writes
//! leaves one of them describing nothing: the body would sit on disk with no
//! metadata to find it by, forever. And raw rather than base64, which is what
//! earns the layout — base64 costs a third of every byte.
//!
//! # Freshness
//!
//! [`Cache-Control`](https://www.rfc-editor.org/rfc/rfc9111#section-5.2) is
//! obeyed where the server gave one. Where it did not, the cache asks rather
//! than assumes: a response carrying an `ETag` or a `Last-Modified` is
//! revalidated with a conditional request, and a `304` costs a few hundred
//! bytes — cheaper than resending the body and cheaper than guessing a
//! lifetime. The mcimirror proxy sends an `ETag` and nothing else, and every
//! request through here is answered by one.
//!
//! # What is refused
//!
//! Only `http` and `https`, and only to an address that is not the user's own
//! network — see `is_reachable`. These URLs are written by whoever published
//! the mod, since they are whatever a Modrinth README or a CurseForge
//! description points at. This is the line between fetching an image and
//! reaching into the machine the launcher runs on.

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

/// How long a network fetch may take. The shared client sets no timeout of its
/// own, so a host that accepts the connection and then says nothing would hold
/// a card's icon open indefinitely — and this is the path that fills a whole
/// page of them.
const TIMEOUT: Duration = Duration::from_secs(15);

/// The largest body written to disk.
///
/// The largest thing that goes through here in normal use is a Minecraft news
/// cover, around 70 kB, or a wiki screenshot at a few megabytes. Past this an
/// entry is served but not stored: a cache is not worth a disk, and a response
/// too big to be a thumbnail is not one a refresh should be holding on to.
const MAX_ITEM_BYTES: usize = 16 * 1024 * 1024;

/// What the whole directory may occupy.
///
/// Sized for a long browsing session's worth of everything: Modrinth's full icon
/// set runs to tens of megabytes at the median icon size this project measures,
/// and a wiki or news image is orders of magnitude larger than that. Past this
/// the oldest entries go.
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;

/// What a prune cuts down to.
///
/// Pruning to exactly [`MAX_TOTAL_BYTES`] would re-run on the very next write;
/// four fifths amortizes the walk over the writes that follow.
const PRUNE_TO: u64 = MAX_TOTAL_BYTES / 5 * 4;

/// Where the entries live. Until the app points this somewhere the cache is
/// inert and every [`fetch`] is a plain request — which is also what keeps it
/// out of the way in a test.
static CACHE_DIR: OnceCell<PathBuf> = OnceCell::new();

/// Points the cache at a directory, creating it if it is not there.
///
/// Settled once, like [`crate::set_system_proxy`]: both have to be in place
/// before the first request and neither can be changed afterwards. Nothing here
/// reads `config`, so a missing directory is not something the user has to
/// configure — it is somewhere to put files.
pub fn set_dir(dir: PathBuf) {
    if let Err(error) = std::fs::create_dir_all(&dir) {
        warn!("Failed to create the HTTP cache directory {dir:?}: {error}");
        return;
    }
    let _ = CACHE_DIR.set(dir);
}

/// The bytes of `url`: from the cache when the copy there is still current, from
/// the network otherwise.
///
/// `None` for anything this cache will not serve — an empty URL, a `data:` URL,
/// a scheme other than `http`/`https`, a host the user's own network answers
/// for, and a request that failed. Callers that can decode nothing from the
/// result draw their fallback, so which of those it was is only worth a log
/// line.
///
/// A `data:` URL is not so much refused as not this cache's business: it is a
/// payload the caller already holds, and decoding one is the caller's to do
/// because only the caller knows which base64 padding to expect.
pub async fn fetch(url: &str) -> Option<Vec<u8>> {
    get(url, false).await
}

/// [`fetch`], ignoring what the cache has and asking the server.
///
/// The way out of an entry that is on disk but will not decode — a truncated
/// write, a body that was never an image — since re-reading it would only
/// produce the same bytes again. What comes back replaces the entry, so this is
/// also how a user-visible refresh would be built.
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

    // Everything past this point touches the network, so this is the first
    // moment the URL has to be one we are willing to send. A cache hit never
    // gets here, which is why an entry that is merely stale costs no lookup.
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
        // "What you have is current." Keep the body but take the new validators
        // and the new clock: a server that rotates its ETag without changing the
        // body would otherwise have this ask again on every single fetch.
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
            // `warn`, not `debug`: a released log has to be able to say *why* an
            // icon is missing, and the status is the one fact that separates a
            // proxy error page from the upstream refusing. A 429 is called out
            // because it is the one answer that says "come back later".
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
    // Read the headers before the body: `bytes` consumes the response, and the
    // validators are what make the next call cheap.
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

/// The cached copy, for a request that did not land.
///
/// A stale icon beats no icon, which is the one case where answering for a
/// failed fetch is better than reporting the failure.
fn stale(cached: Option<Entry>) -> Option<Vec<u8>> {
    cached.map(|entry| entry.bytes)
}

/// Writes an entry and keeps the directory inside its limit.
///
/// A body over [`MAX_ITEM_BYTES`] is returned to the caller but not kept: the cap
/// belongs on the storage, not on the request, and a response too big to be a
/// thumbnail is not one a later refresh should re-read.
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

/// Puts the cached entry's validators on a request, so the server can answer
/// `304` instead of resending the body.
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

/// One cached response: the body as it came off the wire, and the headers that
/// say whether it still is.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    /// `Cache-Control: max-age=…`, in seconds. `None` where the server gave no
    /// explicit freshness, which is not the same as zero: such an entry is
    /// revalidated rather than served straight away.
    max_age: Option<u64>,
    /// `ETag`, verbatim. It is an opaque token, and reformatting it would only
    /// turn a `304` into a resend.
    etag: Option<String>,
    /// `Last-Modified`, verbatim, for the same reason.
    last_modified: Option<String>,
    /// When this body was received, in unix seconds — the age that
    /// [`Entry::is_fresh`] measures against.
    stored_at: u64,
    /// The body, after the metadata length prefix.
    #[serde(skip)]
    bytes: Vec<u8>,
}

impl Entry {
    /// Whether this body can be served without asking the server.
    fn is_fresh(&self, now: u64) -> bool {
        let Some(max_age) = self.max_age else {
            return false;
        };
        // A clock that has gone backwards says nothing about the entry's age, so
        // the safe reading is "ask". `saturating_sub` would turn a negative age
        // into zero and call the entry brand new, which is the one answer a
        // backwards clock must not produce.
        now >= self.stored_at && now - self.stored_at < max_age
    }
}

/// A header as text, dropping one that is not — it cannot be sent back, and a
/// lossy version of a validator only turns a `304` into a resend.
fn header_of(
    headers: &reqwest::header::HeaderMap,
    name: &reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

/// The `max-age` of a `Cache-Control` header, ignoring every other directive.
///
/// A `no-store` arrives here as no `max-age`, which lands on the revalidate path
/// rather than being stored silently — the entry exists but is never served
/// without asking, which is as close to not storing it as a cache that also
/// answers offline can get.
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
    // Named after the URL's own hash rather than the URL itself: a path
    // component like Modrinth's base62 id is case-sensitive, and macOS and
    // Windows collapse case — `data/k8iIgzXE/…` and `data/k8iigzxe/…` would
    // otherwise share one file and show the wrong icon.
    Some(dir.join(format!("{:x}", Sha512::digest(url.as_bytes()))))
}

/// Reads one entry, or `None` if there is nothing usable there.
///
/// A file that vanishes mid-run is ordinary, so its I/O error is `debug`; a file
/// that is there and malformed is `warn`, because that is the one that repeats
/// forever.
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

/// What this process believes the directory holds.
///
/// Counted in memory rather than derived from the directory on every write,
/// because that is a `stat` per entry per fetch and this is the path a page of
/// cards walks. It can drift — a user deleting files behind our back, or a
/// second launcher instance — but only towards being wrong about the total, and
/// the next prune re-measures it.
struct Usage {
    /// `None` until something has measured the directory.
    bytes: Option<u64>,
}

/// Only ever held for arithmetic — never across an `.await`, and never while the
/// filesystem is being walked.
static USAGE: Lazy<Mutex<Usage>> = Lazy::new(|| Mutex::new(Usage { bytes: None }));

/// Cuts the directory back to [`PRUNE_TO`] once it is over [`MAX_TOTAL_BYTES`].
///
/// Runs after a write, so at the limit nothing happens at all and the walk is
/// paid only when entries are actually going out — which, at 128 MB against
/// kilobyte entries, is every few hundred fetches. That is why it can afford to
/// be a plain `read_dir` rather than a maintained index.
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
    // `while let Ok(Some(..))` ends the walk on a read error as well as on the
    // end of the directory, and a truncated walk *under*-counts the usage — so
    // the folder then looks smaller than it is and pruning stops happening. The
    // error is named so that is visible rather than looking like a small cache.
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
    // Oldest first, so the front of the list is what goes.
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

/// Records a write against the running total, seeding it from the directory the
/// first time. Called after every successful write.
fn note_written(size: u64) {
    let Ok(mut usage) = USAGE.lock() else {
        return;
    };
    match &mut usage.bytes {
        Some(bytes) => *bytes = bytes.saturating_add(size),
        None => *usage = Usage { bytes: Some(size) },
    }
}

/// Whether the launcher may send this URL.
///
/// These URLs are written by whoever published the mod, so this is the line
/// between fetching an image and reaching into the machine the launcher runs
/// on. A name that resolves to `127.0.0.1` or to a LAN address is the whole of
/// it, and it needs no exploit: the description simply says so.
///
/// A hostname is resolved first and *every* address it resolves to is checked,
/// because one name can point anywhere, and a name that answers publicly today
/// can answer privately on the next lookup.
///
/// The address is not pinned to the connection that follows, so a hostile
/// server could still rebind between this lookup and reqwest's own. Closing
/// that needs the resolver and the client to share one answer, which is more
/// than this is worth: rebinding needs a name the launcher was going to fetch
/// from anyway, and the two ways it is actually abused — a literal private
/// address, and a name that resolves to one — are caught here.
async fn is_reachable(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        debug!("Refusing {url}: not a URL");
        return false;
    };
    if !scheme_allows(&parsed) {
        // A refusal is a security decision, so `warn` rather than the `debug` the
        // other refusals use: a release log has to be able to show that a URL was
        // rejected for naming something other than http(s).
        warn!("Refusing {url}: only http and https are fetched");
        return false;
    }
    let Some(host) = parsed.host_str() else {
        warn!("Refusing {url}: it names no host to resolve");
        return false;
    };
    // `Url` keeps the brackets around an IPv6 host literal.
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
            // A name that resolves to nothing is not a reason to refuse; it is
            // a request that will fail on its own, and refusing it here would
            // lose the cached copy along the way.
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
