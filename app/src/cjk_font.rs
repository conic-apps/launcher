// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Names a real CJK family for the Han script, so Chinese text does not depend
//! on the OS picking a sensible fallback.
//!
//! `ui/theme.slint` embeds only "Comfortaa Nunito" — Comfortaa's Latin with the
//! Nunito digits merged in — and Slint's built-in fallback chain behind
//! `default-font-family` is `[SansSerif, SystemUi]`. fontique maps those two
//! generic families to hard-coded names, and on Windows they are *Arial* and
//! *Segoe UI* — neither of which has a single Han glyph. Chinese therefore
//! reaches the OS-level script fallback, where fontique asks DirectWrite for a
//! font covering one sample character and uses the answer for the whole script.
//!
//! On Windows that answer is *Yu Gothic UI*, a **Japanese** font, and it has no
//! glyph for 91 of the 364 distinct CJK characters the `zh_CN` catalog uses
//! (动, 戏, 图, 环, 级, 组, 统 …), so those render blank — typically in the middle
//! of otherwise fine text, which is why it reads as "some characters are
//! missing". macOS (CoreText) and Linux (fontconfig) answer with a
//! script-appropriate CJK font and were never affected.
//!
//! Registering the families we want for `Hani`/`Hant` makes the choice explicit
//! instead of leaving it to the OS. Replaces rather than appends to the OS
//! answer, so a single sentence never mixes two different Han fonts.

/// Applies the Han-script fallback for the given bundled locale (e.g. `zh_CN`).
///
/// Safe to call again after a language change; the last call wins. Must run on
/// the thread that owns the Slint global context (the main thread), which is
/// where both the startup call and the settings callback live.
pub fn apply_cjk_fallbacks(locale: &str) {
    #[cfg(target_os = "windows")]
    windows::apply(locale);

    // Elsewhere the OS fallback is already correct, and on Linux fontconfig
    // honours the user's own fontconfig configuration, so leave it alone.
    #[cfg(not(target_os = "windows"))]
    let _ = locale;
}

#[cfg(target_os = "windows")]
mod windows {
    use slint::fontique_011::fontique::{Collection, FallbackKey, FamilyId, Script};

    /// Simplified Chinese families, best first. The first one installed wins;
    /// the tail keeps the result sane on a Windows install without YaHei.
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

    pub(super) fn apply(locale: &str) {
        let mut collection = slint::fontique_011::shared_collection();

        // zh_TW is the only bundled locale that wants Traditional glyph forms.
        // Everything else — including `en_US` and the language picker, which
        // hardcodes 简体中文/繁體語言 regardless of the UI language — is served
        // by the Simplified list, because a missing glyph is a far worse failure
        // than a shared ideograph drawn in its mainland form.
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
        let families: Vec<FamilyId> = names
            .iter()
            .filter_map(|name| collection.family_id(name))
            .collect();

        if families.is_empty() {
            // Nothing we know about is installed. Keep the OS answer rather than
            // clearing the slot, which would leave Han with no font at all.
            log::warn!("no CJK font among {names:?}; keeping the OS Han fallback");
            return;
        }

        collection.set_fallbacks(FallbackKey::new(script, None), families.iter().copied());
        log::debug!(target: "app", "Han fallback set from {names:?}");
    }

    fn hani_script() -> Script {
        Script::from_bytes(*b"Hani")
    }

    fn hant_script() -> Script {
        Script::from_bytes(*b"Hant")
    }
}
