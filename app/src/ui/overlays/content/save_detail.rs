// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The save details overlay: the page a click on a save card opens, over
//! `ui/overlays/content/save-details-panel.slint`.
//!
//! Everything the page lists comes from the summary the saves grid was built
//! from, so the two cannot disagree about a world. Only the picture is read here,
//! because the parse deliberately leaves it alone.

use super::*;

/// Opens the overlay on one of the saves the grid is showing.
pub(crate) fn open_save_detail(ui: &App, folder: &str) {
    let instance = ui.global::<GameState>().get_current_id().to_string();
    let Some(levels) = cache::saves(&instance) else {
        return;
    };
    let Some((folder, level)) = levels
        .iter()
        .find(|(name, _)| name == folder)
        .map(|(name, level)| (name.clone(), level))
    else {
        return;
    };

    let spawn = level.spawn;
    let state = ui.global::<SaveDetailState>();
    state.set_detail_icon(Image::default());
    state.set_detail(SaveDetail {
        folder: SharedString::from(folder.as_str()),
        name: SharedString::from(level.name.as_deref().unwrap_or(&folder)),
        instance_id: SharedString::from(instance.as_str()),
        has_last_played: level.last_played.is_some(),
        last_played_text: SharedString::from(
            level
                .last_played
                .and_then(last_played_text)
                .unwrap_or_default(),
        ),
        has_spawn: spawn.is_some(),
        spawn_x: spawn.map_or(0, |pos| pos[0]),
        spawn_z: spawn.map_or(0, |pos| pos[2]),
        chips: ModelRc::from(Rc::new(VecModel::from(chips(level, &folder)))),
    });
    state.set_open(true);

    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        // A `data:` URL, decoded off the event loop. The grid's own projection
        // has usually left it in the icon cache already, so `resolve_icon` is a
        // lookup rather than a second decode.
        let icon = content::saves::get_save_icon(&instance, &folder)
            .await
            .ok()
            .and_then(|data| fetch_icon(&data));
        crate::ui::services::report::report(&weak, move |ui| {
            let state = ui.global::<SaveDetailState>();
            // Another save was opened while this one's picture was being read.
            if state.get_detail().folder != folder.as_str() {
                return;
            }
            if let Some(icon) = icon.and_then(resolve_icon).or_else(unknown_icon) {
                state.set_detail_icon(icon);
            }
        });
    });
}

/// Dismisses the overlay, leaving the grid underneath it where it was. The detail
/// itself is kept: the panel still draws it until its leave animation ends.
pub(crate) fn close_save_detail(ui: &App) {
    let state = ui.global::<SaveDetailState>();
    if state.get_open() {
        state.set_open(false);
    }
}

/// The pills the page draws under the world's name: how the world is played,
/// what it was played with, and where it lives. A fact the summary does not have
/// is a pill the page does not draw.
fn chips(level: &content::saves::LevelSummary, folder: &str) -> Vec<SaveChip> {
    let mut chips = Vec::new();
    let mut push = |kind: &str, key: &str, text: &str| {
        chips.push(SaveChip {
            kind: SharedString::from(kind),
            key: SharedString::from(key),
            text: SharedString::from(text),
        })
    };

    let game_mode = game_mode_key(level.game_type);
    if !game_mode.is_empty() {
        push("game-mode", game_mode, "");
    }
    if level.allow_commands {
        push("cheats", "", "");
    }
    if let Some(version) = &level.version {
        push("version", "", version);
    }
    push("folder", "", folder);
    chips
}

/// `Data.LastPlayed` in the user's own time zone. The game writes the instant in
/// milliseconds, which is exactly what `chrono` converts from.
fn last_played_text(timestamp: u64) -> Option<String> {
    let millis = i64::try_from(timestamp).ok()?;
    let instant = chrono::DateTime::from_timestamp_millis(millis)?;
    Some(
        instant
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
    )
}
