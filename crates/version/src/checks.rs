// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use platform::{OsArch, OsFamily, PLATFORM_INFO};
use regex::Regex;
use serde_json::Value;

/// Checks whether the rules allow the entry on the current OS platform and
/// enabled features. Matching rules are applied in order, so the last match
/// wins; an empty rule list is allowed, a non-empty one defaults to disallowed.
pub(crate) fn check_allowed(rules: Vec<Value>, enabled_features: &[String]) -> bool {
    // by default it's allowed
    if rules.is_empty() {
        return true;
    }
    // else it's disallow by default
    let mut allow = false;
    for rule in rules {
        let action = if let Some(action) = rule["action"].as_str() {
            action == "allow"
        } else {
            continue;
        };
        let os_passed = check_os(&rule);
        let features_passed = check_features(&rule, enabled_features);
        if os_passed && features_passed {
            allow = action
        }
    }
    allow
}

pub(crate) fn check_os(rule: &Value) -> bool {
    let normalized_os_family = match PLATFORM_INFO.os_family {
        OsFamily::Windows => "windows",
        OsFamily::Linux => "linux",
        OsFamily::Macos => "osx",
    };
    if let Some(os) = rule["os"].as_object() {
        let name_check_passed = if let Some(name) = os.get("name") {
            if let Some(name) = name.as_str() {
                normalized_os_family == name
            } else {
                true
            }
        } else {
            true
        };
        let version_check_passed = if let Some(version) = os.get("version") {
            if let Some(version) = version.as_str() {
                let regex = match Regex::new(version) {
                    Ok(regex) => regex,
                    Err(_) => return false,
                };
                regex.is_match(&PLATFORM_INFO.os_version.to_string())
            } else {
                true
            }
        } else {
            true
        };
        let normalized_arch = match PLATFORM_INFO.arch {
            OsArch::X64 => "x64",
            OsArch::X86 => "x86",
            OsArch::Mips => "mips",
            OsArch::PowerPC => "powerpc",
            OsArch::PowerPC64 => "powerpc64",
            OsArch::Arm => "arm",
            OsArch::Aarch64 => "aarch64",
            OsArch::Unknown => "unknown",
        };
        let arch_check_passed = if let Some(arch) = os.get("arch") {
            if let Some(arch) = arch.as_str() {
                normalized_arch == arch
            } else {
                true
            }
        } else {
            true
        };
        name_check_passed && version_check_passed && arch_check_passed
    } else {
        true
    }
}

pub(crate) fn check_features(rule: &Value, enabled_features: &[String]) -> bool {
    if let Some(features) = rule["features"].as_object() {
        let mut enabled_features_iter = enabled_features.iter();
        features
            .iter()
            .filter(|x| enabled_features_iter.any(|y| x.0 == y) && x.1.as_bool().unwrap_or(false))
            .collect::<Vec<_>>()
            .len()
            == features.len()
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn allowed(rules: Vec<Value>) -> bool {
        check_allowed(rules, &[])
    }

    #[test]
    fn no_rules_means_allowed() {
        assert!(allowed(Vec::new()));
    }

    #[test]
    fn an_unmatched_non_empty_rule_list_is_disallowed() {
        // A rule with no action is skipped, but the non-empty list still
        // defaults to disallowed.
        assert!(!allowed(vec![json!({ "os": { "name": "windows" } })]));
    }

    #[test]
    fn the_last_matching_rule_wins() {
        assert!(allowed(vec![
            json!({ "action": "disallow" }),
            json!({ "action": "allow" }),
        ]));
        assert!(!allowed(vec![
            json!({ "action": "allow" }),
            json!({ "action": "disallow" }),
        ]));
    }

    #[test]
    fn a_feature_rule_needs_the_feature_enabled() {
        let rules = || vec![json!({ "action": "allow", "features": { "demo": true } })];
        assert!(!check_allowed(rules(), &[]));
        assert!(check_allowed(rules(), &["demo".to_string()]));
    }

    #[test]
    fn a_feature_disabled_by_the_rule_never_matches() {
        let rule = json!({ "features": { "demo": false } });
        assert!(!check_features(&rule, &["demo".to_string()]));
        // An empty feature map is vacuously satisfied.
        assert!(check_features(&json!({ "features": {} }), &[]));
        // No features key at all is always satisfied.
        assert!(check_features(&json!({ "action": "allow" }), &[]));
    }

    #[test]
    fn the_os_rule_is_checked_against_this_machine() {
        let current = match PLATFORM_INFO.os_family {
            OsFamily::Windows => "windows",
            OsFamily::Linux => "linux",
            OsFamily::Macos => "osx",
        };
        assert!(check_os(&json!({ "os": { "name": current } })));
        assert!(!check_os(
            &json!({ "os": { "name": "definitely-not-this-os" } })
        ));
    }

    #[test]
    fn a_rule_without_an_os_key_matches_everywhere() {
        assert!(check_os(&json!({ "action": "allow" })));
    }
}
