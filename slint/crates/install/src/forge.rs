// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Forge version list (`crates/install/src/forge.rs`).
//!
//! Only the version list is mirrored; downloading and running the installer
//! belongs to the install task, which is not migrated yet.

use std::{cmp::Reverse, collections::HashMap};

use serde::{Deserialize, Serialize};

use crate::{HTTP_CLIENT, error::*};

/// A list of Forge versions for a given Minecraft version.
#[derive(Clone, Deserialize, Serialize)]
pub struct ForgeVersionList(HashMap<String, Vec<String>>);

impl ForgeVersionList {
    /// Fetches the Forge version list for a specified Minecraft version.
    ///
    /// # Arguments
    ///
    /// * `mcversion` - The target Minecraft version (e.g., "1.20.1").
    ///
    /// # Returns
    ///
    /// A `ForgeVersionList` containing all available Forge versions for the specified Minecraft version.
    pub async fn new() -> Result<Self> {
        let mut list: Self = HTTP_CLIENT
            .get("https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json")
            .send()
            .await?
            .json()
            .await?;
        for versions in list.0.values_mut() {
            versions.sort_by_cached_key(|version| Reverse(tokenize_version(version)));
        }
        Ok(list)
    }

    /// The Forge versions of a Minecraft version, newest first.
    ///
    /// `None` when the list has no entry for that Minecraft version at all
    /// (the frontend's `response[mcVersion]` lookup).
    pub fn get(&self, mcversion: &str) -> Option<&[String]> {
        self.0.get(mcversion).map(Vec::as_slice)
    }
}

/// A token of a Forge version string used for natural ordering.
///
/// Runs of digits compare numerically, everything else lexicographically,
/// so e.g. `36.0.10` correctly sorts after `36.0.9`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum VersionToken {
    Num(u64),
    Text(String),
}

/// Splits a Forge version string into [`VersionToken`]s.
///
/// `.` and `-` act as separators; digit runs become [`VersionToken::Num`],
/// all other runs become [`VersionToken::Text`] (branch suffixes such as
/// `-1.7.10` or `-prerelease`).
fn tokenize_version(version: &str) -> Vec<VersionToken> {
    fn push(tokens: &mut Vec<VersionToken>, run: &mut String) {
        if run.is_empty() {
            return;
        }
        if run.chars().all(|c| c.is_ascii_digit()) {
            let num: u64 = run.parse().unwrap_or(u64::MAX);
            tokens.push(VersionToken::Num(num));
        } else {
            tokens.push(VersionToken::Text(run.clone()));
        }
        run.clear();
    }

    let mut tokens = Vec::new();
    let mut run = String::new();
    for c in version.chars() {
        match c {
            '.' | '-' => push(&mut tokens, &mut run),
            _ => run.push(c),
        }
    }
    push(&mut tokens, &mut run);
    tokens
}
