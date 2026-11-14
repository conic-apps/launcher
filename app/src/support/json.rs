// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Reading the API clients' `serde_json::Value` trees, where every field is
//! optional: a missing or mistyped one is empty, never an error.

use serde_json::Value;

/// `value[name]` as a string, or empty when it is absent or not a string.
pub(crate) fn string(value: &Value, name: &str) -> String {
    value
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
