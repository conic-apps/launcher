// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use platform::PLATFORM_INFO;
use regex::Regex;
use serde_json::Value;

/// Check if all the rules in Rule[] are acceptable in certain OS platform and features.
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
    if let Some(os) = rule["os"].as_object() {
        let name_check_passed = if let Some(name) = os.get("name") {
            if let Some(name) = name.as_str() {
                PLATFORM_INFO.os_family.to_string() == name
            } else {
                true
            }
        } else {
            true
        };
        let version_check_passed = if let Some(version) = os.get("version") {
            if let Some(version) = version.as_str() {
                Regex::is_match(
                    &Regex::new(version).unwrap(),
                    (PLATFORM_INFO.os_version.to_string()).as_ref(),
                )
            } else {
                true
            }
        } else {
            true
        };
        let arch_check_passed = if let Some(arch) = os.get("arch") {
            if let Some(arch) = arch.as_str() {
                PLATFORM_INFO.arch == arch
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
