// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde_json::Value;

use crate::checks::check_allowed;

pub(crate) fn resolve_arguments(args: &[Value], enabled_features: &[String]) -> Vec<String> {
    args.iter()
        .flat_map(|arg| resolve_argument(arg, enabled_features))
        .collect()
}

fn resolve_argument(argument: &Value, enabled_features: &[String]) -> Vec<String> {
    if argument.is_string() {
        match argument.as_str() {
            Some(x) => return vec![x.to_string()],
            None => return vec![],
        }
    }
    let rules = match argument["rules"].as_array() {
        Some(x) => x.clone(),
        None => return vec![],
    };
    if check_allowed(rules, enabled_features) {
        if argument["value"].is_array() {
            serde_json::from_value::<Vec<String>>(argument["value"].clone()).unwrap_or_default()
        } else if argument["value"].is_string() {
            match argument["value"].as_str() {
                Some(x) => vec![x.to_string()],
                None => vec![],
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}
