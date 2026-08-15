// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Configures the in-game language by writing `options.txt` before the first launch.
//!
//! The game directory of an instance is `instances/<id>`, where Minecraft reads
//! `options.txt` on startup. Before the first launch the file does not exist, so it
//! is created during installation with the `lang` key matching the launcher UI
//! language.

use std::path::Path;

use log::info;

use crate::error::Result;

/// The `lang` value casing required by different Minecraft eras:
/// - 1.0 and earlier (released ≤ 2011-11-18): no language option exists at all.
/// - 1.1 ~ 1.10 (released ≤ 2016-06-08): the region must be uppercase (`zh_CN`,
///   `pt_BR`); a lowercase region crashes the game or silently falls back to English.
/// - 1.11+ (released after 2016-06-08): the region must be lowercase (`zh_cn`,
///   `pt_br`); an uppercase region silently falls back to English.
const MC_1_1_RELEASE_DATE: &str = "2011-11-18";
const MC_1_11_RELEASE_DATE: &str = "2016-06-08";

/// The language eras a Minecraft release date can belong to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LanguageEra {
    /// 1.0 and earlier: the game has no language option at all.
    Pre1Dot1,
    /// 1.1 ~ 1.10: the region code must be uppercase.
    Legacy,
    /// 1.11+: the region code must be lowercase.
    Modern,
}

/// Maps a launcher UI language to its Minecraft language code used in `options.txt`.
///
/// The launcher supports 12 languages, each using the exact code of Minecraft's
/// modern language files: `en_us`, `zh_cn`, `zh_tw`, `ja_jp`, `ko_kr`, `de_de`,
/// `fr_fr`, `es_es`, `pt_br`, `ru_ru`, `tr_tr`, `pl_pl`.
fn launcher_language_to_game_code(language: &str) -> &'static str {
    match language {
        "en_us" => "en_us",
        "zh_cn" => "zh_cn",
        "zh_tw" => "zh_tw",
        "ja_jp" => "ja_jp",
        "ko_kr" => "ko_kr",
        "de_de" => "de_de",
        "fr_fr" => "fr_fr",
        "es_es" => "es_es",
        "pt_br" => "pt_br",
        "ru_ru" => "ru_ru",
        "tr_tr" => "tr_tr",
        "pl_pl" => "pl_pl",
        _ => "en_us",
    }
}

/// Determines the language era of a Minecraft version from its ISO-8601 release time.
fn language_era(release_time: Option<&str>) -> LanguageEra {
    let Some(date) = release_time.and_then(|time| time.get(..10)) else {
        return LanguageEra::Modern;
    };
    if date <= MC_1_1_RELEASE_DATE {
        LanguageEra::Pre1Dot1
    } else if date <= MC_1_11_RELEASE_DATE {
        LanguageEra::Legacy
    } else {
        LanguageEra::Modern
    }
}

/// Resolves the final `lang` value for `options.txt`.
///
/// Returns `None` for versions without a language option.
fn resolve_game_language_code(launcher_language: &str, era: LanguageEra) -> Option<String> {
    let code = launcher_language_to_game_code(launcher_language);
    match era {
        LanguageEra::Pre1Dot1 => None,
        LanguageEra::Legacy => {
            let (language, region) = code.split_once('_')?;
            Some(format!("{language}_{}", region.to_uppercase()))
        }
        LanguageEra::Modern => Some(code.to_string()),
    }
}

/// Whether the game needs `forceUnicodeFont` so CJK characters render properly.
fn needs_force_unicode_font(language_code: &str) -> bool {
    language_code.starts_with("zh_")
        || language_code.starts_with("ja_")
        || language_code.starts_with("ko_")
}

/// Writes `options.txt` with the game language matching the launcher UI language.
///
/// Only runs when the file does not exist yet (the first launch), so a language
/// chosen inside the game is never overwritten, even on reinstall.
pub async fn configure_game_language(
    options_txt_path: &Path,
    launcher_language: &str,
    release_time: Option<&str>,
) -> Result<()> {
    if options_txt_path.exists() {
        info!("options.txt already exists, keeping the in-game language");
        return Ok(());
    }
    let Some(language_code) =
        resolve_game_language_code(launcher_language, language_era(release_time))
    else {
        info!("Minecraft version has no language option, skipping");
        return Ok(());
    };
    let mut content = format!("lang:{language_code}\n");
    if needs_force_unicode_font(&language_code) {
        content.push_str("forceUnicodeFont:true\n");
    }
    async_fs::write(options_txt_path, content.as_bytes()).await?;
    info!("Set the game language to {language_code}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_supported_languages() {
        for code in [
            "en_us", "zh_cn", "zh_tw", "ja_jp", "ko_kr", "de_de", "fr_fr", "es_es", "pt_br",
            "ru_ru", "tr_tr", "pl_pl",
        ] {
            assert_eq!(launcher_language_to_game_code(code), code);
        }
        assert_eq!(launcher_language_to_game_code("unknown"), "en_us");
    }

    #[test]
    fn detects_language_eras() {
        assert_eq!(
            language_era(Some("2011-11-18T12:00:00+00:00")),
            LanguageEra::Pre1Dot1
        );
        assert_eq!(
            language_era(Some("2012-01-12T12:00:00+00:00")),
            LanguageEra::Legacy
        );
        assert_eq!(
            language_era(Some("2015-07-31T15:30:00+00:00")),
            LanguageEra::Legacy
        );
        assert_eq!(
            language_era(Some("2016-06-08T10:38:00+00:00")),
            LanguageEra::Legacy
        );
        assert_eq!(
            language_era(Some("2016-11-14T12:00:00+00:00")),
            LanguageEra::Modern
        );
        assert_eq!(
            language_era(Some("2021-06-08T10:38:00+00:00")),
            LanguageEra::Modern
        );
        assert_eq!(language_era(None), LanguageEra::Modern);
    }

    #[test]
    fn resolves_version_specific_casing() {
        assert_eq!(
            resolve_game_language_code("zh_cn", LanguageEra::Legacy),
            Some("zh_CN".to_string())
        );
        assert_eq!(
            resolve_game_language_code("zh_cn", LanguageEra::Modern),
            Some("zh_cn".to_string())
        );
        assert_eq!(
            resolve_game_language_code("pt_br", LanguageEra::Legacy),
            Some("pt_BR".to_string())
        );
        assert_eq!(
            resolve_game_language_code("de_de", LanguageEra::Legacy),
            Some("de_DE".to_string())
        );
        assert_eq!(
            resolve_game_language_code("zh_cn", LanguageEra::Pre1Dot1),
            None
        );
    }

    #[test]
    fn cjk_needs_unicode_font() {
        assert!(needs_force_unicode_font("zh_cn"));
        assert!(needs_force_unicode_font("ja_jp"));
        assert!(needs_force_unicode_font("ko_kr"));
        assert!(!needs_force_unicode_font("en_us"));
    }
}
