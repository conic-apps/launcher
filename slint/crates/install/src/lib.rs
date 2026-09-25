// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Tauri-free mirror of `crates/install`: the Minecraft version metadata a
//! launcher needs before it can install anything.
//!
//! The original crate exposes these through `async` Tauri commands that cache
//! their answers in the plugin state. This mirror keeps the same shape:
//!
//!   * its functions are `async` and use the shared async HTTP client, so they
//!     are awaited on a tokio runtime like the original's commands;
//!   * it owns the caches itself, standing in for the Tauri `PluginState`;
//!   * it carries the version-list half of the crate only. The install task
//!     itself (game files, loaders, Java) arrives with the launch view — see
//!     `slint/README.md`.
//!
//! Structures, request URLs and sorting match `crates/install/src/*.rs` so the
//! two crates can be diffed against each other.

use std::{
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{info, warn};
use once_cell::sync::{Lazy, OnceCell};

mod error;
pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod quilt;
pub mod vanilla;

pub use error::*;

use fabric::LoaderArtifactList;
use forge::ForgeVersionList;
use quilt::QuiltVersionList;
use vanilla::VersionManifest;

/// How long a fetched version list stays fresh, mirroring
/// `CACHE_EXPIRATION_SECONDS` in `crates/install/src/lib.rs`.
static CACHE_EXPIRATION_SECONDS: u64 = 1800;

/// Whether requests should go through the OS proxy settings. Mirrors
/// `shared::SHOULD_USE_SYSTEM_PROXY` — the Tauri app sets it from the config
/// (`crates/config/src/lib.rs`), so the Slint app does the same at startup.
/// Unset means "use the system proxy", like the original's fallback.
pub static SHOULD_USE_SYSTEM_PROXY: OnceCell<bool> = OnceCell::new();

/// Records the config's proxy preference. Must run before the first request
/// (the client below is built on first use and cannot be reconfigured).
pub fn set_system_proxy(use_system_proxy: bool) {
    let _ = SHOULD_USE_SYSTEM_PROXY.set(use_system_proxy);
}

/// The shared HTTP client, standing in for `shared::HTTP_CLIENT`.
///
/// `shared::HTTP_CLIENT` also forces rustls, which is unnecessary here: the
/// workspace enables no other TLS backend.
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let should_use_system_proxy = match SHOULD_USE_SYSTEM_PROXY.get() {
        Some(should_use_system_proxy) => *should_use_system_proxy,
        None => {
            warn!(
                "Unable to determine whether to use the system proxy; using the default settings."
            );
            true
        }
    };
    if should_use_system_proxy {
        info!("Using system proxy");
    } else {
        info!("Shouldn't use system proxy");
    }
    let mut builder = reqwest::ClientBuilder::new()
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(200)
        .user_agent(format!("ConicApps/{}", env!("CARGO_PKG_VERSION")));
    if !should_use_system_proxy {
        builder = builder.no_proxy();
    }
    builder.build().expect("Failed to build HTTP client")
});

/// Seconds since the epoch, used as the cache timestamp.
fn unix_now() -> u64 {
    // A clock before the epoch would make every entry look stale, which is
    // harmless (it just refetches) — unlike the original's `expect`, which
    // would take the worker thread down with it.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The cached copy of a version list, if the last fetch is still fresh.
///
/// `crates/install` keeps these in its Tauri `PluginState`; the freshness test
/// here is the intended one (the original compares the age the other way round,
/// so it only ever serves a copy that is *older* than the TTL).
fn cached<T: Clone>(cache: &Mutex<Option<(u64, T)>>) -> Option<T> {
    let guard = cache.lock().expect("Internal error");
    let (fetched_at, value) = guard.as_ref()?;
    (unix_now().saturating_sub(*fetched_at) < CACHE_EXPIRATION_SECONDS).then(|| value.clone())
}

/// Stores a freshly fetched list and hands it back.
fn store<T: Clone>(cache: &Mutex<Option<(u64, T)>>, value: T) -> T {
    *cache.lock().expect("Internal error") = Some((unix_now(), value.clone()));
    value
}

static MANIFEST_CACHE: Lazy<Mutex<Option<(u64, VersionManifest)>>> = Lazy::new(|| Mutex::new(None));
static FORGE_VERSION_LIST_CACHE: Lazy<Mutex<Option<(u64, ForgeVersionList)>>> =
    Lazy::new(|| Mutex::new(None));
// The original carries the same allowance on the field of its `PluginState`.
#[allow(clippy::type_complexity)]
static NEOFORGE_VERSION_LIST_CACHE: Lazy<Mutex<Option<(u64, Vec<String>)>>> =
    Lazy::new(|| Mutex::new(None));

/// Every Minecraft version Mojang knows, newest first (`cmd_get_minecraft_version_list`).
pub async fn get_minecraft_version_list() -> Result<VersionManifest> {
    if let Some(cached) = cached(&MANIFEST_CACHE) {
        return Ok(cached);
    }
    Ok(store(&MANIFEST_CACHE, VersionManifest::new().await?))
}

/// The Fabric loader versions for a Minecraft version
/// (`cmd_get_fabric_version_list`). Not cached: the answer depends on the
/// Minecraft version and the original caches it per plugin instance only.
pub async fn get_fabric_version_list(mcversion: &str) -> Result<LoaderArtifactList> {
    LoaderArtifactList::new(mcversion).await
}

/// The Quilt loader versions for a Minecraft version
/// (`cmd_get_quilt_version_list`). Not cached, like Fabric.
pub async fn get_quilt_version_list(mcversion: &str) -> Result<QuiltVersionList> {
    QuiltVersionList::new(mcversion).await
}

/// Every Forge version, keyed by Minecraft version
/// (`cmd_get_forge_version_list`).
pub async fn get_forge_version_list() -> Result<ForgeVersionList> {
    if let Some(cached) = cached(&FORGE_VERSION_LIST_CACHE) {
        return Ok(cached);
    }
    Ok(store(
        &FORGE_VERSION_LIST_CACHE,
        ForgeVersionList::new().await?,
    ))
}

/// Every Neoforge version, newest first (`cmd_get_neoforge_version_list`).
pub async fn get_neoforge_version_list() -> Result<Vec<String>> {
    if let Some(cached) = cached(&NEOFORGE_VERSION_LIST_CACHE) {
        return Ok(cached);
    }
    Ok(store(
        &NEOFORGE_VERSION_LIST_CACHE,
        neoforge::get_neoforge_version_list().await?,
    ))
}
