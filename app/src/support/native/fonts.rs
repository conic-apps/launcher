// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A symbol font behind the Han fallback.
//!
//! fontique resolves fallbacks per script, and `Common` symbols match none, so
//! parley's appended `Hani` fallback is all they reach. These families go after
//! the Han ones, so Han text keeps its font and only its missing characters
//! reach a symbol font.

use slint::fontique_011::fontique::{FallbackKey, FamilyId, Script};

/// Symbol families, best first; every installed one is appended.
#[cfg(target_os = "macos")]
const SYMBOLS: &[&str] = &["Apple Symbols"];

#[cfg(target_os = "windows")]
const SYMBOLS: &[&str] = &["Segoe UI Symbol", "Cambria Math"];

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const SYMBOLS: &[&str] = &[
    "Noto Sans Symbols 2",
    "Noto Sans Symbols",
    "DejaVu Sans",
    "Symbola",
];

/// Appends the symbol families to the `Hani` fallback, keeping the Han families
/// already there in front. Idempotent: a family already in the list is not added
/// twice.
pub(crate) fn apply() {
    let mut collection = slint::fontique_011::shared_collection();
    let key = FallbackKey::new(Script::from_bytes(*b"Hani"), None);

    // Collected because the iterator borrows the collection `family_id` needs.
    let mut families: Vec<FamilyId> = collection.fallback_families(key).collect();
    let mut resolved = 0;
    let mut added = 0;

    for name in SYMBOLS {
        let Some(id) = collection.family_id(name) else {
            log::debug!(target: "app", "the symbol family {name:?} is not installed");
            continue;
        };
        resolved += 1;
        if families.contains(&id) {
            continue;
        }
        families.push(id);
        added += 1;
    }

    if resolved == 0 {
        log::warn!(
            target: "app",
            "no symbol font among {SYMBOLS:?}; missing symbols keep rendering as .notdef"
        );
        return;
    }

    if added > 0 {
        collection.set_fallbacks(key, families.iter().copied());
        log::debug!(target: "app", "the Han fallback gained {added} symbol family/families");
    }
}
