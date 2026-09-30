// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance list's own state, kept between runs (`src/store/instance.ts`
//! and `InstancesList.vue`'s `localStorage`).
//!
//! Four things survive a restart in the Vue, none of them in `config.toml`:
//! which instance was selected (`currentInstanceId`), the list's sort order
//! (`instancesSortMode`), its grouping (`instancesGroupMode`) and which groups
//! were left open (`instancesGroupExpanded`). They went to the webview's local
//! storage, which is per-webview and therefore invisible to anything else.
//!
//! A native app has no such store, so the same four go to a file next to
//! `config.toml` in the shared data directory — the arrangement
//! `slint-music`'s `session` already established for the player. It is one of
//! the few things the two frontends cannot share: the Tauri app cannot read this
//! file, and this one cannot read the webview's storage, so a launcher that is
//! run one way and then the other starts from the defaults.
//!
//! What is read back is deliberately forgiving. The Vue wrote each of the three
//! scalars straight out and checked the value against its own list of modes on
//! the way in, and wrapped the group map's `JSON.parse` in a `try` that
//! discarded anything that was not an object of booleans; the same is done here,
//! so a file written by an older build, or edited by hand, costs the defaults
//! and nothing else.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use slint_folder::DATA_LOCATION;

/// The file the state is kept in, under the data directory's root.
fn state_file() -> PathBuf {
    DATA_LOCATION.root.join("instance_view.json")
}

/// How the list is ordered (`InstancesList.vue`'s `SortMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Version,
    #[default]
    Playtime,
    LastPlay,
}

impl SortMode {
    /// The one place the two spellings meet: the webview wrote `"lastplay"`
    /// straight into `localStorage`, while the listing's own enum spells it
    /// `LastPlayed`.
    ///
    /// The Vue checked a read value against its list of modes
    /// (`SORT_MODES.includes(…)`) and fell back to `"playtime"` for anything
    /// else. The saved file carries the raw string and the check happens here
    /// rather than in `serde`, because a value that does not parse costs *this*
    /// one setting and nothing else — the Vue's read was per value too.
    pub fn from_key(key: &str) -> Self {
        match key {
            "name" => Self::Name,
            "version" => Self::Version,
            "lastplay" => Self::LastPlay,
            _ => Self::default(),
        }
    }
}

/// How the list is grouped (`InstancesList.vue`'s `GroupMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroupMode {
    #[default]
    None,
    Loader,
}

impl GroupMode {
    /// The `GROUP_MODES.includes(…)` of the original, with the same per-value
    /// fallback.
    pub fn from_key(key: &str) -> Self {
        match key {
            "loader" => Self::Loader,
            _ => Self::default(),
        }
    }
}

/// The four saved values.
///
/// Each is a string rather than an enum on purpose: the Vue read its two modes
/// with an `includes` check and its group map inside a `try`, so a hand-edited or
/// out-of-date file lost the one value that was wrong and kept the rest. A
/// `serde`-derived enum would take the whole file down with it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InstanceViewState {
    /// The selected instance's id, `""` when there is none.
    #[serde(default)]
    pub current_id: String,
    /// The sort order, as the webview spelled it: "name" | "version" |
    /// "playtime" | "lastplay".
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
    /// defaults, which is `music_session.rs`'s `load` and the Vue's
    /// `loadExpanded` both doing.
    pub fn from_slice(raw: &[u8]) -> Self {
        serde_json::from_slice(raw).unwrap_or_default()
    }
}

/// Writes the state.
///
/// The Vue's three `localStorage.setItem` calls cannot fail in a way the user
/// would see; here a failure is logged and otherwise ignored, so a read-only or
/// full data directory costs the saved state and nothing else.
pub fn save(state: &InstanceViewState) {
    let path = state_file();
    let Ok(bytes) = serde_json::to_vec_pretty(state) else {
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
/// Every failure — a missing file, a truncated write, a mode that is not one of
/// the four — falls back to the default for that one value, and the rest of the
/// file is still used. The Vue discarded the whole group map on a parse failure
/// but read its two scalars through `includes`, which is what this matches.
pub fn load() -> InstanceViewState {
    let Ok(raw) = std::fs::read(state_file()) else {
        return InstanceViewState::default();
    };
    InstanceViewState::from_slice(&raw)
}
