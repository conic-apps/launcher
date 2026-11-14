// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Configures the in-game language by writing `lang` into `options.txt`.
//!
//! [`configure_game_language`] writes it during install only when the game has no
//! options file; [`force_game_language`] is the modpack path, where an override's
//! `options.txt` keeps its other options but loses `lang`. The language must exist
//! in the client jar.

use std::path::Path;

use log::{info, warn};
use version::MinecraftVersion;

use crate::error::Result;

/// The `lang` value casing required by different Minecraft eras:
/// - 1.0 and earlier (released ≤ 2011-11-18): no language option exists at all.
/// - 1.1 ~ 1.10 (released ≤ 2016-06-08): the region must be uppercase (`zh_CN`,
///   `pt_BR`); a lowercase region crashes the game or silently falls back to English.
/// - 1.11+ (released after 2016-06-08): the region must be lowercase (`zh_cn`,
///   `pt_br`); an uppercase region silently falls back to English.
///
/// The dates are only consulted for snapshots, whose id has no release number
/// to compare; releases use their version number instead.
const MC_1_1_RELEASE_DATE: &str = "2011-11-18";
const MC_1_10_RELEASE_DATE: &str = "2016-06-08";

/// The language eras a Minecraft release can belong to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LanguageEra {
    /// 1.0 and earlier: the game has no language option at all.
    Pre1Dot1,
    Legacy,
    Modern,
}

/// Maps a launcher UI language code to the Minecraft code used in
/// `options.txt`; `None` for a code with no game equivalent, which is left for
/// the game's own default rather than forced to English.
fn launcher_language_to_game_code(language: &str) -> Option<&'static str> {
    match language {
        "en_us" => Some("en_us"),
        "zh_cn" => Some("zh_cn"),
        "zh_tw" => Some("zh_tw"),
        "ja_jp" => Some("ja_jp"),
        "ko_kr" => Some("ko_kr"),
        "de_de" => Some("de_de"),
        "fr_fr" => Some("fr_fr"),
        "es_es" => Some("es_es"),
        "pt_br" => Some("pt_br"),
        "ru_ru" => Some("ru_ru"),
        "tr_tr" => Some("tr_tr"),
        "pl_pl" => Some("pl_pl"),
        _ => None,
    }
}

/// The era a version belongs to, or `None` when it cannot be decided.
///
/// Releases are compared by version number, so `1.10.1` and `1.10.2` stay in
/// the uppercase era even though they were released after the `1.10` date.
/// Snapshots have no comparable number and fall back to `release_time`; an
/// unknown id or a snapshot without a usable date is not guessed at.
fn language_era(version_id: &str, release_time: Option<&str>) -> Option<LanguageEra> {
    match version_id.parse::<MinecraftVersion>() {
        Ok(MinecraftVersion::Release(major, minor, _)) => Some(if (major, minor) < (1, 1) {
            LanguageEra::Pre1Dot1
        } else if (major, minor) < (1, 11) {
            LanguageEra::Legacy
        } else {
            LanguageEra::Modern
        }),
        Ok(MinecraftVersion::Snapshot(..)) => era_from_release_time(release_time),
        Ok(MinecraftVersion::Unknown(_)) | Err(_) => None,
    }
}

fn era_from_release_time(release_time: Option<&str>) -> Option<LanguageEra> {
    let date = release_time.and_then(|time| time.get(..10))?;
    Some(if date <= MC_1_1_RELEASE_DATE {
        LanguageEra::Pre1Dot1
    } else if date <= MC_1_10_RELEASE_DATE {
        LanguageEra::Legacy
    } else {
        LanguageEra::Modern
    })
}

/// Resolves the final `lang` value for `options.txt`, or `None` for versions
/// without a language option and launcher languages with no game equivalent.
fn resolve_game_language_code(launcher_language: &str, era: LanguageEra) -> Option<String> {
    let code = launcher_language_to_game_code(launcher_language)?;
    match era {
        LanguageEra::Pre1Dot1 => None,
        LanguageEra::Legacy => {
            let (language, region) = code.split_once('_')?;
            Some(format!("{language}_{}", region.to_uppercase()))
        }
        LanguageEra::Modern => Some(code.to_string()),
    }
}

/// The entry names a language may use in the client jar: `.lang` before 1.13
/// and `.json` from 1.13 on, both under the same language directory.
fn language_entry_names(language_code: &str) -> [String; 2] {
    [
        format!("assets/minecraft/lang/{language_code}.lang"),
        format!("assets/minecraft/lang/{language_code}.json"),
    ]
}

async fn language_is_available(client_jar_path: &Path, language_code: &str) -> Result<bool> {
    let jar_path = client_jar_path.to_path_buf();
    let code = language_code.to_string();
    tokio::task::spawn_blocking(move || -> Result<bool> {
        let file = std::fs::File::open(&jar_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        Ok(language_entry_names(&code)
            .iter()
            .any(|entry| archive.by_name(entry).is_ok()))
    })
    .await?
}

/// Whether any `options.txt` exists under `config/`, up to two levels deep.
///
/// A pack or a mod can keep the game's options there, and writing a root
/// `options.txt` next to it would make the launcher's language lose.
async fn options_exists_in_config(instance_root: &Path) -> bool {
    let mut stack = vec![(instance_root.join("config"), 0u8)];
    while let Some((directory, depth)) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                warn!(
                    "Could not read {} while looking for options.txt: {error}",
                    directory.display()
                );
                continue;
            }
        };
        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(error) => {
                    warn!(
                        "Could not read an entry of {}: {error}",
                        directory.display()
                    );
                    break;
                }
            };
            let file_type = match entry.file_type().await {
                Ok(file_type) => file_type,
                Err(error) => {
                    warn!("Could not stat {}: {error}", entry.path().display());
                    continue;
                }
            };
            if file_type.is_file() && entry.file_name().to_str() == Some("options.txt") {
                return true;
            }
            if file_type.is_dir() && depth < 2 {
                stack.push((entry.path(), depth + 1));
            }
        }
    }
    false
}

/// Sets the resolved language in `options.txt`, replacing an existing `lang`
/// line and creating the file when it is absent. Other lines and their order are
/// kept, so a pack's options survive.
async fn write_language(options_txt_path: &Path, language_code: &str) -> Result<()> {
    let existing = match tokio::fs::read_to_string(options_txt_path).await {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let mut lines: Vec<String> = existing
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let mut replaced = false;
    for line in &mut lines {
        if line.starts_with("lang:") {
            *line = format!("lang:{language_code}");
            replaced = true;
            break;
        }
    }
    if !replaced {
        lines.insert(0, format!("lang:{language_code}"));
    }
    let mut content = lines.join("\n");
    content.push('\n');
    tokio::fs::write(options_txt_path, content.as_bytes()).await?;
    Ok(())
}

/// Resolves the language to write, or `None` when the version has no language
/// option, the launcher language has no game equivalent, or the client jar does
/// not ship that language.
async fn resolve_available_language(
    launcher_language: &str,
    version_id: &str,
    release_time: Option<&str>,
    client_jar_path: &Path,
) -> Result<Option<String>> {
    let Some(language_code) = language_era(version_id, release_time)
        .and_then(|era| resolve_game_language_code(launcher_language, era))
    else {
        return Ok(None);
    };
    if !language_is_available(client_jar_path, &language_code).await? {
        info!("Minecraft {version_id} has no {language_code} language; leaving the game default");
        return Ok(None);
    }
    Ok(Some(language_code))
}

/// Writes `options.txt` with the game language matching the launcher UI language.
///
/// Only runs when the file does not exist yet (the first launch), so a language
/// chosen inside the game is never overwritten, even on reinstall.
pub async fn configure_game_language(
    options_txt_path: &Path,
    launcher_language: &str,
    version_id: &str,
    release_time: Option<&str>,
    client_jar_path: &Path,
) -> Result<()> {
    if options_txt_path.exists() {
        info!("options.txt already exists, keeping the in-game language");
        return Ok(());
    }
    if let Some(instance_root) = options_txt_path.parent()
        && options_exists_in_config(instance_root).await
    {
        info!("config/ contains options.txt, keeping the game's own options");
        return Ok(());
    }
    let Some(language_code) =
        resolve_available_language(launcher_language, version_id, release_time, client_jar_path)
            .await?
    else {
        info!("No in-game language to set for Minecraft {version_id}");
        return Ok(());
    };
    write_language(options_txt_path, &language_code).await?;
    info!("Set the game language to {language_code}");
    Ok(())
}

/// Sets the game language even when `options.txt` already exists, replacing
/// only its `lang` line.
///
/// The modpack installer calls this after extracting the pack's overrides: an
/// override can supply `options.txt`, which wins over the install-time write,
/// so the launcher's language has to be applied again on top of it.
pub async fn force_game_language(
    options_txt_path: &Path,
    launcher_language: &str,
    version_id: &str,
    release_time: Option<&str>,
    client_jar_path: &Path,
) -> Result<()> {
    let Some(language_code) =
        resolve_available_language(launcher_language, version_id, release_time, client_jar_path)
            .await?
    else {
        info!("No in-game language to force for Minecraft {version_id}");
        return Ok(());
    };
    write_language(options_txt_path, &language_code).await?;
    info!("Forced the game language to {language_code}");
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
            assert_eq!(launcher_language_to_game_code(code), Some(code));
        }
        assert_eq!(launcher_language_to_game_code("unknown"), None);
        assert_eq!(launcher_language_to_game_code("ar_sa"), None);
    }

    #[test]
    fn detects_language_eras_from_version_ids() {
        assert_eq!(language_era("1.0", None), Some(LanguageEra::Pre1Dot1));
        assert_eq!(language_era("1.1", None), Some(LanguageEra::Legacy));
        assert_eq!(language_era("1.7.10", None), Some(LanguageEra::Legacy));
        assert_eq!(language_era("1.10", None), Some(LanguageEra::Legacy));
        // Regression: 1.10.1 and 1.10.2 were released after the 1.10 date but
        // still need the uppercase region.
        assert_eq!(language_era("1.10.1", None), Some(LanguageEra::Legacy));
        assert_eq!(language_era("1.10.2", None), Some(LanguageEra::Legacy));
        assert_eq!(language_era("1.11", None), Some(LanguageEra::Modern));
        assert_eq!(language_era("1.12.2", None), Some(LanguageEra::Modern));
        assert_eq!(language_era("1.20.1", None), Some(LanguageEra::Modern));
        assert_eq!(language_era("a1.2.6", None), None);
        assert_eq!(language_era("not a version", None), None);
    }

    #[test]
    fn snapshots_fall_back_to_the_release_time() {
        assert_eq!(
            language_era("24w14a", Some("2011-11-18T12:00:00+00:00")),
            Some(LanguageEra::Pre1Dot1)
        );
        assert_eq!(
            language_era("24w14a", Some("2015-07-31T15:30:00+00:00")),
            Some(LanguageEra::Legacy)
        );
        assert_eq!(
            language_era("24w14a", Some("2021-06-08T10:38:00+00:00")),
            Some(LanguageEra::Modern)
        );
        assert_eq!(language_era("24w14a", None), None);
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
        assert_eq!(
            resolve_game_language_code("unknown", LanguageEra::Modern),
            None
        );
    }

    #[test]
    fn names_both_language_file_layouts() {
        assert_eq!(
            language_entry_names("zh_cn"),
            [
                "assets/minecraft/lang/zh_cn.lang".to_string(),
                "assets/minecraft/lang/zh_cn.json".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn replaces_only_the_lang_line() {
        let directory = std::env::temp_dir().join(format!("conic-language-{}", std::process::id()));
        tokio::fs::create_dir_all(&directory)
            .await
            .expect("create the temp directory");
        let path = directory.join("options.txt");
        tokio::fs::write(&path, "fov:70\nlang:en_us\nrenderDistance:8\n")
            .await
            .expect("write the options file");

        write_language(&path, "zh_cn")
            .await
            .expect("set the language");

        let content = tokio::fs::read_to_string(&path)
            .await
            .expect("read the options file");
        assert_eq!(content, "fov:70\nlang:zh_cn\nrenderDistance:8\n");
        let _ = tokio::fs::remove_dir_all(&directory).await;
    }

    #[tokio::test]
    async fn finds_options_under_config() {
        let directory =
            std::env::temp_dir().join(format!("conic-language-config-{}", std::process::id()));
        let nested = directory.join("config").join("pack");
        tokio::fs::create_dir_all(&nested)
            .await
            .expect("create the temp directory");
        tokio::fs::write(nested.join("options.txt"), "fov:70\n")
            .await
            .expect("write the nested options file");

        assert!(options_exists_in_config(&directory).await);
        let _ = tokio::fs::remove_dir_all(&directory).await;
    }
}
