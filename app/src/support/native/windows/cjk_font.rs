// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Names a real CJK family for the Han script, so Chinese text does not depend on
//! the OS picking a sensible fallback — on Windows that fallback is a Japanese
//! font with gaps for some of the characters the `zh_CN` catalog uses.
//!
//! Replaces rather than appends to the OS answer, so a sentence never mixes two
//! Han fonts.

use slint::fontique_011::fontique::{Collection, FallbackKey, FamilyId, Script};

/// Simplified Chinese families, best first; the first installed one wins. The
/// tail keeps the result sane on a Windows install without YaHei.
const SIMPLIFIED: &[&str] = &[
    "Microsoft YaHei UI",
    "Microsoft YaHei",
    "Noto Sans SC",
    "Source Han Sans SC",
    "DengXian",
    "SimSun",
    "NSimSun",
    "SimHei",
    "FangSong",
];

/// Traditional Chinese families, best first.
const TRADITIONAL: &[&str] = &[
    "Microsoft JhengHei UI",
    "Microsoft JhengHei",
    "Noto Sans TC",
    "Source Han Sans TC",
    "PMingLiU",
    "MingLiU",
];

/// Applies the Han-script fallback for the given bundled locale (e.g. `zh_CN`).
///
/// Safe to call again after a language change; the last call wins. Must run on
/// the thread that owns the Slint global context (the main thread), which is
/// where both the startup call and the settings callback live.
pub(crate) fn apply(locale: &str) {
    let mut collection = slint::fontique_011::shared_collection();

    // zh_TW is the only bundled locale that wants Traditional glyph forms.
    // Everything else — including the language picker, which hardcodes
    // 简体中文/繁體中文 — is served by the Simplified list: a missing glyph is a far
    // worse failure than a shared ideograph drawn in its mainland form.
    let hani = if locale == "zh_TW" {
        TRADITIONAL
    } else {
        SIMPLIFIED
    };

    // A nil locale shares fontique's fallback slot with the per-locale `Hani`
    // keys, so this also overrides the `zh-CN` lookup parley would make.
    set_fallback(&mut collection, hani_script(), hani);
    set_fallback(&mut collection, hant_script(), TRADITIONAL);
}

fn set_fallback(collection: &mut Collection, script: Script, names: &[&str]) {
    // The *resolved* families, not the requested names: a missing family is
    // dropped, so logging `names` could claim a font that is not there.
    let resolved: Vec<FamilyId> = names
        .iter()
        .filter_map(|name| {
            let id = collection.family_id(name);
            if id.is_none() {
                log::debug!(target: "app", "the CJK family {name:?} is not installed");
            }
            id
        })
        .collect();

    if resolved.is_empty() {
        // Nothing we know about is installed. Keep the OS answer rather than
        // clearing the slot, which would leave Han with no font at all.
        log::warn!(
            target: "app",
            "no CJK font among {names:?}; keeping the OS Han fallback"
        );
        return;
    }

    collection.set_fallbacks(FallbackKey::new(script, None), resolved.iter().copied());
    log::debug!(
        target: "app",
        "the {script:?} fallback is set from {} family/families out of {} requested",
        resolved.len(),
        names.len()
    );
}

fn hani_script() -> Script {
    Script::from_bytes(*b"Hani")
}

fn hant_script() -> Script {
    Script::from_bytes(*b"Hant")
}
