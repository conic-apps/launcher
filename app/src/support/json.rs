// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Helpers for reading the API clients' `serde_json::Value`s.
//!
//! The remote responses (`modrinth`, `curseforge`, the Yggdrasil server info)
//! are walked by hand, and to this layer every field is optional: a missing one
//! reads as empty rather than failing the whole response. The same
//! `get(name).and_then(as_str).unwrap_or_default()` chain would otherwise be
//! spelled out at every one of those sites.

use serde_json::Value;

/// `value[name]` as a string, or empty when it is absent or not a string.
pub(crate) fn string(value: &Value, name: &str) -> String {
    value
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
