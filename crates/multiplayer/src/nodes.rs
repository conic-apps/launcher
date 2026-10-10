// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The public EasyTier node list, served by the API server so nodes can change
//! without shipping a new build.

use std::time::Duration;

use log::warn;
use serde::Deserialize;
use shared::HTTP_CLIENT;

/// Baked in by `build.rs`. Empty when `BUILD_API_KEY` was unset at build time.
const API_KEY: &str = env!("CONIC_NEXUS_API_KEY");

const NODES_ENDPOINT: &str = "https://api.conicmc.app/easytier/nodes";

/// A node list is not worth delaying the session over, so the request is given
/// its own deadline rather than the shared client's unbounded one.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Only the field the session needs; the server also sends a name, description
/// and region.
#[derive(Deserialize)]
struct Node {
    url: String,
}

/// The `url` of every public node the server advertises.
///
/// A build without a key, an unreachable server or an unparsable body all yield
/// an empty list after a warning: the session can still start and connect
/// peer-to-peer, so none of them is worth failing the caller over.
pub(crate) async fn fetch_public_nodes() -> Vec<String> {
    if API_KEY.is_empty() {
        warn!("No Conic Nexus API key is baked in; the public node list is skipped");
        return Vec::new();
    }

    let response = match HTTP_CLIENT
        .get(NODES_ENDPOINT)
        .header("X-API-Key", API_KEY)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            warn!("The Conic Nexus node list could not be reached: {error}");
            return Vec::new();
        }
    };
    if !response.status().is_success() {
        warn!("The Conic Nexus node list answered {}", response.status());
        return Vec::new();
    }

    match response.json::<Vec<Node>>().await {
        Ok(nodes) => nodes.into_iter().map(|node| node.url).collect(),
        Err(error) => {
            warn!("The Conic Nexus node list did not parse: {error}");
            Vec::new()
        }
    }
}
