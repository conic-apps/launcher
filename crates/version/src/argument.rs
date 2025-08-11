// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::checks::check_allowed;

#[derive(Clone, Deserialize, Serialize)]
pub struct Arguments {
    pub game: Option<Vec<Value>>,
    pub jvm: Option<Vec<Value>>,
}

#[derive(Clone, Serialize, Default)]
pub struct ResolvedArguments {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}

impl Arguments {
    pub(crate) fn to_resolved(&self, enabled_features: &[String]) -> ResolvedArguments {
        let mut resolved_game_args = vec![];
        let mut resolved_jvm_args = vec![];
        if let Some(game_args) = &self.game {
            for arg in game_args {
                resolved_game_args.extend(resolve_argument(arg, enabled_features));
            }
        }
        if let Some(jvm_args) = &self.jvm {
            for arg in jvm_args {
                resolved_jvm_args.extend(resolve_argument(arg, enabled_features));
            }
        }
        ResolvedArguments {
            game: resolved_game_args,
            jvm: resolved_jvm_args,
        }
    }
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
