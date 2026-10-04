// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The update server protocol: the endpoint, the JSON payload it answers with,
//! and the version comparison that decides whether it is worth acting on.
//!
//! The server is a dynamic endpoint (see `conic-apps/conic-updater-server`):
//! `GET {base}/v1/{channel}/{target}/{arch}/{current}?kind={kind}` answers
//! `204` when there is nothing new and `200` with a JSON payload otherwise.

use semver::Version;
use serde::Deserialize;
use shared::UrlExt;
use url::Url;

use crate::error::{Error, Result};
use crate::install::InstallKind;

/// The public update endpoint.
pub const ENDPOINT_BASE: &str = "https://api.conicmc.app/update/v1";

/// The minisign public key (base64 of the key line) every bundle is signed
/// with. Rotating it means every installed copy has to be replaced by hand, so
/// it is a release-engineering decision, not a build one.
///
/// The matching secret lives in the release repository's secrets as
/// `CONIC_UPDATE_SIGNING_KEY` (and its optional password as
/// `CONIC_UPDATE_SIGNING_KEY_PASSWORD`); `update-sign generate` mints a new
/// pair and prints both halves.
pub const PUBLIC_KEY: &str = "RWTDUhkJd0bnxt5d0Ll7w1XxRAzp4BNdeFiHM/5h67BkuA8fjzijW3R9";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        }
    }
}

/// The JSON body a `200` carries.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: String,
    pub url: String,
    pub signature: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

impl UpdateInfo {
    /// Whether this release is strictly newer than the running one. An
    /// unparsable pair is treated as "not newer" rather than as an error: a
    /// local build may carry a version the server cannot order.
    pub fn is_newer_than(&self, current: &str) -> bool {
        match (
            Version::parse(self.version.trim_start_matches(['v', 'V'])),
            Version::parse(current.trim_start_matches(['v', 'V'])),
        ) {
            (Ok(candidate), Ok(running)) => candidate > running,
            (candidate, running) => {
                log::warn!(
                    "could not compare update version {:?} against {running:?}; ignoring",
                    candidate.map(|v| v.to_string())
                );
                false
            }
        }
    }
}

/// The `target` path segment the server expects.
pub fn target() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

/// The `arch` path segment the server expects.
pub fn arch() -> &'static str {
    std::env::consts::ARCH
}

/// Builds the check URL for one channel, install form and running version.
pub fn endpoint(channel: Channel, kind: InstallKind, current: &str) -> Result<Url> {
    let mut url = Url::parse(ENDPOINT_BASE)?;
    url = url.append_path([channel.as_str(), target(), arch(), current])?;
    url.query_pairs_mut().append_pair("kind", kind.as_str());
    Ok(url)
}

/// Asks the server whether a newer bundle for `kind` exists.
pub async fn check(
    channel: Channel,
    kind: InstallKind,
    current: &str,
) -> Result<Option<UpdateInfo>> {
    let url = endpoint(channel, kind, current)?;
    log::debug!("checking for updates: {url}");
    let response = shared::HTTP_CLIENT.get(url).send().await?;
    match response.status().as_u16() {
        // The server's "nothing to update" answer.
        204 => Ok(None),
        200 => {
            let info: UpdateInfo = response.json().await?;
            if info.is_newer_than(current) {
                Ok(Some(info))
            } else {
                Ok(None)
            }
        }
        status => Err(Error::Status(status)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_release_wins_and_an_older_one_does_not() {
        let info = |version: &str| UpdateInfo {
            version: version.to_string(),
            notes: String::new(),
            pub_date: String::new(),
            url: String::new(),
            signature: String::new(),
            sha256: None,
            size: None,
        };
        assert!(info("0.1.0").is_newer_than("0.1.0-alpha.2"));
        assert!(info("0.1.0-beta.1").is_newer_than("0.1.0-alpha.2"));
        assert!(!info("0.1.0-alpha.2").is_newer_than("0.1.0-alpha.2"));
        assert!(!info("0.0.9").is_newer_than("0.1.0"));
        // A leading `v` is accepted on either side.
        assert!(info("v0.2.0").is_newer_than("0.1.0"));
    }

    #[test]
    fn an_unparsable_version_is_not_newer() {
        let info = UpdateInfo {
            version: "nightly".to_string(),
            notes: String::new(),
            pub_date: String::new(),
            url: String::new(),
            signature: String::new(),
            sha256: None,
            size: None,
        };
        assert!(!info.is_newer_than("0.1.0"));
        assert!(!info.is_newer_than("also-not-semver"));
    }

    #[test]
    fn the_endpoint_carries_the_kind_query() {
        let url = endpoint(Channel::Beta, InstallKind::AppImage, "0.1.0-alpha.2")
            .expect("a valid endpoint");
        assert!(url.path().contains("/v1/beta/"), "path was {}", url.path());
        assert!(url.path().ends_with("/0.1.0-alpha.2"));
        assert!(url.query().is_some_and(|q| q.contains("kind=appimage")));
    }
}
