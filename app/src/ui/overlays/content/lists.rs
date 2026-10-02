// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The local lists: saves, mods, resource packs, screenshots and favourites.

use super::*;

pub(crate) fn load_saves(ui: &App) {
    ui.global::<ContentState>().set_saves_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::support::runtime::spawn(async move {
        // Gzip and NBT parsing is blocking, so it runs on the blocking pool.
        let levels = crate::support::runtime::spawn_blocking({
            let instance = instance.clone();
            move || {
                content::saves::get_all_levels(&instance).map(|levels| {
                    levels
                        .into_iter()
                        .map(|(folder, root)| (folder, content::saves::summarize_level(&root)))
                        .collect::<Vec<_>>()
                })
            }
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();

        let mut cards = Vec::new();
        for (folder, level) in levels {
            let icon = content::saves::get_save_icon(&instance, &folder)
                .await
                .ok()
                .and_then(|data| fetch_icon(&data));
            cards.push(save_card(&folder, &level, icon));
        }
        cards.sort_by(|a, b| a.id.cmp(&b.id));

        crate::ui::services::report::report(&weak, move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for card in &cards {
                    state
                        .targets
                        .insert(card.id.to_string(), CardTarget::Save(card.id.to_string()));
                }
            }
            set_cards(&ui, Grid::Saves, cards);
            ui.global::<ContentState>().set_saves_loading(false);
        });
    });
}

/// One save's card, from the summary read out of `level.dat`.
pub(crate) fn save_card(
    folder: &str,
    level: &content::saves::LevelSummary,
    icon: Option<PendingImage>,
) -> PendingCard {
    let name = level.name.clone().unwrap_or_else(|| folder.to_string());
    let cheats = level.allow_commands;
    let last_played = level.last_played;

    let mut tags: Vec<PendingTag> = Vec::new();
    if let Some(game_type) = level.game_type {
        // The four names are words the user reads, so they are resolved in the
        // card (`ContentText.save-tag`) rather than composed here: a string Rust
        // pushed is fixed at that moment and would not follow a language change.
        let kind = match game_type {
            0 => "game-mode-survival",
            1 => "game-mode-creative",
            2 => "game-mode-adventure",
            3 => "game-mode-spectator",
            _ => "",
        };
        if !kind.is_empty() {
            tags.push(translated_tag(kind));
        }
    }
    if cheats {
        tags.push(translated_tag("command-enabled"));
    }
    if last_played.is_some() {
        // The label and the relative time are both translated, so neither can
        // be composed here; the tag carries the parts `GameTime.last-played`
        // needs and the card resolves them.
        let relative = crate::ui::views::game::relative_time(last_played);
        tags.push(PendingTag {
            text: String::new(),
            label: String::new(),
            kind: "last-played".to_string(),
            time: Some((
                relative.kind,
                relative.hours,
                relative.month,
                relative.day,
                relative.year,
            )),
        });
    }

    PendingCard {
        id: folder.to_string(),
        title: name,
        subtitle: folder.to_string(),
        has_subtitle: true,
        tags,
        icon,
        action_kind: "file",
        shows_play: true,
        // The save's spawn point, handed to the world map as its center. `None`
        // where there is no `Data.spawn.pos`, which is the same as `(0, 0)` here
        // and asks the world for its own spawn.
        spawn: level.spawn.map(|pos| (pos[0], pos[2])),
        ..Default::default()
    }
}

/// The plain tags: a fill and a text, with no label of their own.
pub(crate) fn tag(text: &str, kind: &str) -> PendingTag {
    PendingTag {
        text: text.to_string(),
        label: String::new(),
        kind: kind.to_string(),
        time: None,
    }
}

/// A tag whose *text* is a word the user reads and the card has to resolve:
/// the saves' game-mode and cheats chips. Only the kind travels, and
/// `ContentText.save-tag` turns it into the sentence in the current language.
pub(crate) fn translated_tag(kind: &str) -> PendingTag {
    PendingTag {
        text: String::new(),
        label: String::new(),
        kind: kind.to_string(),
        time: None,
    }
}

pub(crate) fn load_local_mods(ui: &App) {
    ui.global::<ContentState>().set_local_mods_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::support::runtime::spawn(async move {
        let mods = content::mods::remote::parse_mods(&instance).await;
        let cards: Vec<PendingCard> = mods
            .iter()
            // Embedded (jar-in-jar) mods are filtered out of the list.
            .filter(|mod_info| !mod_info.embedded)
            .map(local_mod_card)
            .collect();
        crate::ui::services::report::report(&weak, move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for card in &cards {
                    state
                        .targets
                        .insert(card.id.to_string(), CardTarget::Path(card.id.to_string()));
                }
            }
            set_cards(&ui, Grid::LocalMods, cards);
            ui.global::<ContentState>().set_local_mods_loading(false);
        });
    });
}

pub(crate) fn local_mod_card(mod_info: &ResolvedMod) -> PendingCard {
    let mut tags: Vec<PendingTag> = Vec::new();
    if mod_info.loader != ModLoader::Unknown {
        let key = loader_key(mod_info.loader);
        tags.push(tag(&capitalize(key), &format!("loader-{key}")));
    }
    if let Some(version) = &mod_info.version {
        tags.push(tag(version, "version"));
    }
    PendingCard {
        id: mod_info.path.to_string_lossy().to_string(),
        title: mod_info.name.clone(),
        subtitle: format!(
            "by {}",
            mod_info
                .authors
                .iter()
                .map(|author| author.name.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ),
        has_subtitle: true,
        description: mod_info.description.clone().unwrap_or_default(),
        tags,
        // A local icon is a base64 data URL and an online one is a plain URL,
        // which is what `merge_remote` leaves behind; both load the same way.
        icon: mod_info.icon.as_deref().and_then(fetch_icon),
        action_kind: "file",
        mod_disabled: mod_info.disabled,
        ..Default::default()
    }
}

pub(crate) fn loader_key(loader: ModLoader) -> &'static str {
    match loader {
        ModLoader::Fabric => "fabric",
        ModLoader::Forge => "forge",
        ModLoader::Quilt => "quilt",
        ModLoader::NeoForge => "neoforge",
        ModLoader::LiteLoader => "liteloader",
        ModLoader::Unknown => "",
    }
}

/// `fabric` → `Fabric`. A loader name is never translated, so this is not a
/// `ContentText` label.
pub(crate) fn capitalize(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn load_local_resourcepacks(ui: &App) {
    ui.global::<ContentState>()
        .set_local_resourcepacks_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::support::runtime::spawn(async move {
        // Reading a pack means opening a zip, so it is blocking work.
        let packs = crate::support::runtime::spawn_blocking({
            let instance = instance.clone();
            move || content::resourcepack::get_instance_resourcepacks(&instance)
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
        let cards: Vec<PendingCard> = packs.iter().map(resourcepack_card).collect();
        crate::ui::services::report::report(&weak, move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for card in &cards {
                    state
                        .targets
                        .insert(card.id.to_string(), CardTarget::Path(card.id.to_string()));
                }
            }
            set_cards(&ui, Grid::LocalResourcePacks, cards);
            ui.global::<ContentState>()
                .set_local_resourcepacks_loading(false);
        });
    });
}

pub(crate) fn resourcepack_card(pack: &content::resourcepack::Resourcepack) -> PendingCard {
    let mut tags: Vec<PendingTag> = Vec::new();
    let min = pack
        .metadata
        .pointer("/pack/min_format/0")
        .and_then(Value::as_i64);
    let max = pack
        .metadata
        .pointer("/pack/max_format/0")
        .and_then(Value::as_i64);
    let range = match (min, max) {
        (Some(min), Some(max)) if min == max => format!("{min}"),
        (Some(min), None) => format!("{min}+"),
        (Some(min), Some(max)) => format!("{min}-{max}"),
        (None, _) => String::new(),
    };
    if !range.is_empty() {
        tags.push(tag(&range, "version"));
    }
    PendingCard {
        id: pack.path.to_string_lossy().to_string(),
        title: pack.name.clone(),
        description: pack
            .metadata
            .pointer("/pack/description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tags,
        icon: pack.icon.as_deref().and_then(fetch_icon),
        action_kind: "file",
        ..Default::default()
    }
}

/// How many icons a preview row shows.
pub(crate) const PREVIEW_ICONS: usize = 5;

/// The four preview rows' icons: the first five saves, mods, resource packs and
/// screenshots of the instance, each decoded here — the game view's rows are a
/// view of the same content the panels show. A save's, a resource pack's and a
/// screenshot's icon is a local file or a data URL; a mod's may be a remote URL,
/// because `parse_mods` merges online info into it.
///
/// Called whenever the current instance changes, as the game view re-reads its
/// counts then.
pub fn refresh_preview_icons(ui: &App, instance_id: &str) {
    let instance = instance_id.to_string();
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let (saves, mods, packs, shots) = crate::support::runtime::spawn_blocking({
            let instance = instance.clone();
            move || {
                let mut saves: Vec<PendingImage> = Vec::new();
                if let Ok(levels) = content::saves::get_all_levels(&instance) {
                    let mut folders: Vec<String> = levels.keys().cloned().collect();
                    folders.sort();
                    for folder in folders.into_iter().take(PREVIEW_ICONS) {
                        // Blocking: it reads the level's `icon.png` and encodes it.
                        let Ok(icon) = crate::support::runtime::block_on(
                            content::saves::get_save_icon(&instance, &folder),
                        ) else {
                            continue;
                        };
                        if let Some(image) = fetch_icon(&icon) {
                            saves.push(image);
                        }
                    }
                }
                // `parse_mods` is the instance-aware one (`parse_folder` wants
                // the folder itself); it is async, so it is driven to
                // completion here — this whole closure is already off the UI
                // thread.
                let mods: Vec<PendingImage> =
                    crate::support::runtime::block_on(content::mods::remote::parse_mods(&instance))
                        .iter()
                        .filter(|mod_info| !mod_info.embedded)
                        .filter_map(|mod_info| mod_info.icon.as_deref())
                        .take(PREVIEW_ICONS)
                        .filter_map(fetch_icon)
                        .collect();
                let packs: Vec<PendingImage> =
                    content::resourcepack::get_instance_resourcepacks(&instance)
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|pack| pack.icon.as_deref())
                        .take(PREVIEW_ICONS)
                        .filter_map(fetch_icon)
                        .collect();
                let shots: Vec<PendingImage> = content::screenshots::list_screenshots(&instance)
                    .unwrap_or_default()
                    .iter()
                    .take(PREVIEW_ICONS)
                    .filter_map(|path| {
                        let bytes = std::fs::read(path).ok()?;
                        let (width, height, rgba) = decode_to_rgba(&bytes)?;
                        Some(PendingImage {
                            key: path.clone(),
                            width,
                            height,
                            rgba,
                        })
                    })
                    .collect();
                (saves, mods, packs, shots)
            }
        })
        .await
        .unwrap_or_default();
        crate::ui::services::report::report(&weak, move |ui| {
            let state = ui.global::<GameState>();
            let images = |pending: Vec<PendingImage>| {
                ModelRc::from(Rc::new(VecModel::from(
                    pending
                        .into_iter()
                        .filter_map(|image| resolve_image(image, &ICONS))
                        .collect::<Vec<Image>>(),
                )))
            };
            state.set_preview_saves(images(saves));
            state.set_preview_mods(images(mods));
            state.set_preview_resourcepacks(images(packs));
            state.set_preview_screenshots(images(shots));
        });
    });
}

pub(crate) fn load_screenshots(ui: &App) {
    ui.global::<ContentState>().set_screenshots_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::support::runtime::spawn(async move {
        let paths = content::screenshots::list_screenshots(&instance).unwrap_or_default();
        let images: Vec<PendingImage> = paths
            .iter()
            .filter_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                let (width, height, rgba) = decode_to_rgba(&bytes)?;
                Some(PendingImage {
                    key: path.clone(),
                    width,
                    height,
                    rgba,
                })
            })
            .collect();
        crate::ui::services::report::report(&weak, move |ui| {
            let shots: Vec<GalleryShot> = images
                .into_iter()
                .filter_map(|image| resolve_gallery_shot_with(image, &SCREENSHOTS))
                .collect();
            let ui_state = ui.global::<ContentState>();
            ui_state.set_screenshots(ModelRc::from(Rc::new(VecModel::from(shots))));
            ui_state.set_screenshots_loading(false);
            ui_state.set_screenshot_index(0);
            if let Some(first) = ui_state.get_screenshots().row_data(0) {
                ui_state.set_current_screenshot(first.image);
            }
        });
    });
}

pub(crate) fn set_screenshot(ui: &App, index: i32) {
    let state = ui.global::<ContentState>();
    let count = state.get_screenshots().row_count() as i32;
    if count == 0 {
        return;
    }
    let index = index.clamp(0, count - 1);
    state.set_screenshot_index(index);
    if let Some(shot) = state.get_screenshots().row_data(index as usize) {
        state.set_current_screenshot(shot.image);
    }
}

/// One card expands and the rest collapse, and the expansion flips upwards when
/// the card sits within 160px of the window's bottom edge.
pub(crate) fn select_save(ui: &App, folder: &str) {
    let state = ui.global::<ContentState>();
    let model = state.get_saves();
    let mut cards: Vec<ContentCard> = (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .collect();
    let flip_threshold = ui.window().size().height as i32 - 160;
    for card in &mut cards {
        let selected = !folder.is_empty() && card.id == folder;
        card.selected = selected;
        card.expand_up = selected && card.y as i32 + card.height as i32 > flip_threshold;
    }
    for (index, card) in cards.into_iter().enumerate() {
        model.set_row_data(index, card);
    }
}

pub(crate) fn refresh_favorite_flags(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let ui_state = ui.global::<ContentState>();
    let model = ui_state.get_remote_cards();
    let mut cards: Vec<ContentCard> = (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .collect();
    let mut changed = false;
    for card in &mut cards {
        let Some(CardTarget::Remote { platform, id }) = state.targets.get(card.id.as_str()) else {
            continue;
        };
        let favorited = state.is_favorited(*platform, id);
        if card.favorited != favorited {
            card.favorited = favorited;
            changed = true;
        }
    }
    if changed {
        for (index, card) in cards.into_iter().enumerate() {
            model.set_row_data(index, card);
        }
    }
    if let Some(detail) = &state.detail {
        ui_state.set_detail_favorited(state.is_favorited(detail.platform, &detail.id));
    }
}
