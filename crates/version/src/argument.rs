// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
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
            // One element that is not a string fails the whole deserialization,
            // and the default is an *empty* list — so a single unexpected token in
            // a loader's json silently removes every argument that entry
            // contributes, `--add-opens` and `-p` included. That is the shape of
            // "the game starts and then dies", so the entry is logged.
            match serde_json::from_value::<Vec<String>>(argument["value"].clone()) {
                Ok(values) => values,
                Err(error) => {
                    log::warn!(
                        "an argument entry contributes nothing: its 'value' is not a list of \
                         strings ({error}) — {}",
                        argument["value"]
                    );
                    Vec::new()
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn resolve(args: Value) -> Vec<String> {
        resolve_arguments(std::slice::from_ref(&args), &[])
    }

    #[test]
    fn a_plain_string_is_itself() {
        assert_eq!(resolve(json!("--demo")), vec!["--demo".to_string()]);
    }

    #[test]
    fn an_allowed_object_with_a_string_value_contributes_it() {
        assert_eq!(
            resolve(json!({ "rules": [{ "action": "allow" }], "value": "--width" })),
            vec!["--width".to_string()]
        );
    }

    #[test]
    fn an_allowed_object_with_an_array_value_contributes_every_entry() {
        assert_eq!(
            resolve(json!({
                "rules": [{ "action": "allow" }],
                "value": ["--a", "--b"],
            })),
            vec!["--a".to_string(), "--b".to_string()]
        );
    }

    #[test]
    fn a_disallowed_object_contributes_nothing() {
        assert!(resolve(json!({ "rules": [{ "action": "disallow" }], "value": "--a" })).is_empty());
    }

    #[test]
    fn an_object_without_rules_contributes_nothing() {
        assert!(resolve(json!({ "value": "--a" })).is_empty());
    }

    #[test]
    fn a_non_string_value_is_ignored() {
        assert!(resolve(json!({ "rules": [{ "action": "allow" }], "value": 1 })).is_empty());
    }

    #[test]
    fn several_arguments_are_flattened_in_order() {
        let resolved = resolve_arguments(
            &[
                json!("--a"),
                json!({ "rules": [{ "action": "allow" }], "value": "--b" }),
                json!("--c"),
            ],
            &[],
        );
        assert_eq!(resolved, vec!["--a", "--b", "--c"]);
    }
}
