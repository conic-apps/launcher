// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance list's own state, kept between runs.
//!
//! Four things survive a restart, none of them in `config.toml`: which instance
//! was selected, the list's sort order, its grouping and which groups were left
//! open. They go to a file next to `config.toml` in the shared data directory,
//! the arrangement `music`'s session already uses for the player.
//!
//! What is read back is deliberately forgiving: a `sort` or `group_mode` string
//! that names no known mode falls back to that setting's default, so a file
//! written by an older build, or edited by hand, loses only that one setting. A
//! file that does not parse at all falls back to the defaults.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use storage::LOCATIONS;

fn state_file() -> PathBuf {
    LOCATIONS.launcher.root.join("instance_view.json")
}

/// How the list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Version,
    #[default]
    Playtime,
    LastPlay,
}

impl SortMode {
    /// Parses a saved sort key, falling back to the default for anything else.
    ///
    /// This is the one place the saved spelling (`lastplay`) meets the enum's
    /// (`LastPlay`). The check happens here rather than in `serde`, because a
    /// value that does not parse costs *this* one setting and nothing else.
    pub fn from_key(key: &str) -> Self {
        match key {
            "name" => Self::Name,
            "version" => Self::Version,
            "lastplay" => Self::LastPlay,
            _ => Self::default(),
        }
    }
}

/// How the list is grouped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroupMode {
    #[default]
    None,
    Loader,
}

impl GroupMode {
    /// Parses a saved group key, falling back to the default per value.
    pub fn from_key(key: &str) -> Self {
        match key {
            "loader" => Self::Loader,
            _ => Self::default(),
        }
    }
}

/// The saved values.
///
/// `sort` and `group_mode` are strings rather than enums on purpose: a
/// hand-edited or out-of-date value loses only that one setting and keeps the
/// rest, where a `serde`-derived enum would take the whole file down with it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InstanceViewState {
    /// The selected instance's id, `""` when there is none.
    #[serde(default)]
    pub current_id: String,
    /// The sort order: "name" | "version" | "playtime" | "lastplay".
    #[serde(default)]
    pub sort: String,
    /// The grouping: "none" | "loader".
    #[serde(default)]
    pub group_mode: String,
    /// Which groups were left open. Only the groups the list actually shows are
    /// in here; the rest are left to their default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expanded: BTreeMap<String, bool>,
}

impl InstanceViewState {
    pub fn sort_mode(&self) -> SortMode {
        SortMode::from_key(&self.sort)
    }

    pub fn group(&self) -> GroupMode {
        GroupMode::from_key(&self.group_mode)
    }

    /// Parses a saved file. A `serde_json` error anywhere in it costs the
    /// defaults.
    pub fn from_slice(raw: &[u8]) -> Self {
        serde_json::from_slice(raw).unwrap_or_default()
    }
}

/// Writes the state.
///
/// A write cannot fail in a way the user would see, so a failure is logged and
/// otherwise ignored: a read-only or full data directory costs the saved state
/// and nothing else.
pub fn save(state: &InstanceViewState) {
    let path = state_file();
    let Ok(bytes) = serde_json::to_vec_pretty(state) else {
        log::warn!("the instance view state could not be serialised; it is not being saved");
        return;
    };
    if let Err(error) = std::fs::write(&path, bytes) {
        log::warn!(
            "could not save the instance view state to {}: {error}",
            path.display()
        );
    }
}

/// Reads the state back, or the defaults when there is none to read.
///
/// A missing or unparseable file — a truncated write — costs the defaults. A
/// `sort` or `group_mode` value that names no known mode costs only that one
/// setting, and the rest of the file is still used.
pub fn load() -> InstanceViewState {
    let Ok(raw) = std::fs::read(state_file()) else {
        return InstanceViewState::default();
    };
    InstanceViewState::from_slice(&raw)
}
