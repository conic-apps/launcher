// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The local lists: saves, mods, resource packs, screenshots and favourites.
//!
//! None of these read the instance. They turn what [`cache`] parsed into cards,
//! so opening a panel is a walk over a list that is already in memory — the
//! parse happened when the game view asked for the summary's counts.

use super::*;

/// The saves grid, from the shared cache.
///
/// A save's icon is read here rather than by the parse: the summary needs the
/// world's name and game mode but not its picture, and reading an `icon.png` out
/// of every world to show five of them is the kind of work that should follow
/// the panel, not the selection.
pub(crate) fn project_saves(ui: &App) {
    let instance = current_instance(ui);
    let Some(levels) = cache::saves(&instance) else {
        return;
    };
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let mut cards = Vec::new();
        for (folder, level) in levels.iter() {
            let icon = content::saves::get_save_icon(&instance, folder)
                .await
                .ok()
                .and_then(|data| fetch_icon(&data));
            cards.push(save_card(folder, level, icon));
        }
        crate::ui::services::report::report(&weak, move |ui| {
            for card in &cards {
                controller()
                    .borrow_mut()
                    .targets
                    .insert(card.id.to_string(), CardTarget::Save(card.id.to_string()));
            }
            set_cards(&ui, Grid::Saves, cards);
        });
    });
}

fn current_instance(ui: &App) -> String {
    ui.global::<GameState>().get_current_id().to_string()
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

/// The mods grid, from the shared cache.
///
/// The jar-in-jar mods are gone before they got here: [`cache::mods`] filters
/// them, which is the same filter the summary's count came from.
pub(crate) fn project_local_mods(ui: &App) {
    let Some(mods) = cache::mods(&current_instance(ui)) else {
        return;
    };
    let cards: Vec<PendingCard> = mods.iter().map(local_mod_card).collect();
    report_local(ui, Grid::LocalMods, cards);
}

/// What every local grid does with the cards it built: the row's file is the
/// target of its actions, so they go in the same report that puts the cards in
/// the model — a relayout reads `targets` for the row it is drawing.
fn report_local(ui: &App, grid: Grid, cards: Vec<PendingCard>) {
    let weak = ui.as_weak();
    crate::ui::services::report::report(&weak, move |ui| {
        let state = controller();
        let mut state = state.borrow_mut();
        for card in &cards {
            state
                .targets
                .insert(card.id.to_string(), CardTarget::Path(card.id.to_string()));
        }
        drop(state);
        set_cards(&ui, grid, cards);
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
        // Neither is fetched here — `set_cards` fetches them after the grid is
        // on screen, so the list appears while its icons are still arriving.
        icon: None,
        icon_url: mod_info.icon.clone(),
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

pub(crate) fn project_local_resourcepacks(ui: &App) {
    let Some(packs) = cache::resourcepacks(&current_instance(ui)) else {
        return;
    };
    let cards: Vec<PendingCard> = packs.iter().map(resourcepack_card).collect();
    report_local(ui, Grid::LocalResourcePacks, cards);
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

/// The screenshot gallery, from the shared cache.
///
/// Every screenshot is decoded, not just the five the summary draws — the panel
/// is a gallery and a person pages through all of them. The decodes go through
/// the same memo the previews read, so a screenshot that was on the summary
/// costs nothing here.
pub(crate) fn project_screenshots(ui: &App) {
    let Some(paths) = cache::screenshots(&current_instance(ui)) else {
        return;
    };
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let images: Vec<PendingImage> = paths
            .iter()
            .filter_map(|path| Memo::Screenshots.fetch(&path.to_string_lossy()))
            .collect();
        crate::ui::services::report::report(&weak, move |ui| {
            let shots: Vec<GalleryShot> = images
                .into_iter()
                .filter_map(|image| resolve_gallery_shot_with(image, &SCREENSHOTS))
                .collect();
            let ui_state = ui.global::<ContentState>();
            ui_state.set_screenshots(ModelRc::from(Rc::new(VecModel::from(shots))));
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
