// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The grids the panels lay their cards out in.

use super::*;

/// Lays the cards out and returns the grid's height.
///
/// The cards are positioned by hand — Slint's `GridLayout` does not wrap and a
/// wrapping `FlexboxLayout` is measured at its own preferred width — and doing
/// it here also means a resize rewrites the model with `set_row_data` and the
/// same elements move, keeping their hover and their decoded icons, where a
/// rebuilt model would re-create every one of them.
pub(crate) fn layout(cards: &mut [ContentCard], grid_width: i32, card_height: i32) -> i32 {
    let content = (grid_width - 2 * GRID_PAD_X).max(0);
    let columns = (((content + GRID_GAP) / (GRID_MIN_CARD + GRID_GAP)).max(1)) as usize;
    let card_width = ((content - (columns as i32 - 1) * GRID_GAP) / columns as i32).max(0);
    for (index, card) in cards.iter_mut().enumerate() {
        let column = (index % columns) as i32;
        let row = (index / columns) as i32;
        card.x = (CARD_SHIFT + column * (card_width + GRID_GAP)) as f32;
        card.y = (row * (card_height + GRID_GAP)) as f32;
        card.width = card_width as f32;
        card.height = card_height as f32;
        card.in_view = true;
    }
    let rows = cards.len().div_ceil(columns).max(1) as i32;
    GRID_PAD_TOP + rows * card_height + (rows - 1) * GRID_GAP + GRID_PAD_BOTTOM
}

/// The model a grid kind draws into.
pub(crate) fn grid_model(ui: &App, grid: Grid) -> ModelRc<ContentCard> {
    let state = ui.global::<ContentState>();
    match grid {
        Grid::Saves => state.get_saves(),
        Grid::LocalMods => state.get_local_mods(),
        Grid::LocalResourcePacks => state.get_local_resourcepacks(),
        Grid::Remote => state.get_remote_cards(),
    }
}

pub(crate) fn grid_card_height(grid: Grid) -> i32 {
    match grid {
        Grid::Saves => CARD_HEIGHT_SAVES,
        Grid::LocalResourcePacks => CARD_HEIGHT_NO_SUBTITLE,
        Grid::LocalMods | Grid::Remote => CARD_HEIGHT,
    }
}

/// Replaces a grid's cards, laying them out for the width the panel has.
///
/// Runs on the UI thread — the cards become Slint structs here, because a model
/// cannot be made anywhere else.
pub(crate) fn set_cards(ui: &App, grid: Grid, pending: Vec<PendingCard>) {
    let (width, panel_height, model) = {
        let state = controller();
        let mut state = state.borrow_mut();
        state.open_grid = Some(grid);
        (state.grid_width, state.panel_height, state.model(grid))
    };
    let ui_state = ui.global::<ContentState>();
    let mut cards: Vec<ContentCard> = pending.into_iter().map(PendingCard::finish).collect();
    let height = if cards.is_empty() {
        // Nothing to place: give the placeholder the room the grid would have
        // taken, so it sits where the cards would have been.
        (panel_height - TITLE_BAR_HEIGHT).max(0)
    } else {
        layout(&mut cards, width, grid_card_height(grid))
    };
    set_grid_height(&ui_state, grid, height);
    if model.row_count() != cards.len() {
        model.set_vec(cards);
    } else {
        for (index, card) in cards.into_iter().enumerate() {
            model.set_row_data(index, card);
        }
    }
}

/// Makes `grid` the one on screen: it becomes the grid a resize re-lays out,
/// and it is re-laid out now, because a grid that was loaded while another list
/// was open still carries the positions it was given at the old panel width.
pub(crate) fn show_grid(ui: &App, grid: Grid) {
    controller().borrow_mut().open_grid = Some(grid);
    relayout(ui);
}

/// Writes one grid's height to that grid's own property: the four grids are
/// laid out independently, and the one on screen must not be given another
/// list's height — see the note on the properties in `globals/content.slint`.
pub(crate) fn set_grid_height(state: &ContentState<'_>, grid: Grid, height: i32) {
    let height = height as f32;
    match grid {
        Grid::Saves => state.set_saves_grid_height(height),
        Grid::LocalMods => state.set_local_mods_grid_height(height),
        Grid::LocalResourcePacks => state.set_local_resourcepacks_grid_height(height),
        Grid::Remote => state.set_remote_grid_height(height),
    }
}

/// Re-runs the open grid's layout. Called when a panel reports a new width — a
/// window resize, or a panel opening at a different size.
pub(crate) fn relayout(ui: &App) {
    let (grid, width, height) = {
        let state = controller();
        let state = state.borrow();
        let Some(grid) = state.open_grid else {
            return;
        };
        (grid, state.grid_width, state.panel_height)
    };
    let model = grid_model(ui, grid);
    let mut cards: Vec<ContentCard> = (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .collect();
    let grid_height = if cards.is_empty() {
        (height - TITLE_BAR_HEIGHT).max(0)
    } else {
        layout(&mut cards, width, grid_card_height(grid))
    };
    set_grid_height(&ui.global::<ContentState>(), grid, grid_height);
    if !cards.is_empty() {
        for (index, card) in cards.into_iter().enumerate() {
            model.set_row_data(index, card);
        }
    }
}
