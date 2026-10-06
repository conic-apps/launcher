// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The news overlay's callbacks, registered on the `App`, `NewsState` and
//! `NewsSearch` globals.
//!
//! Each callback upgrades the one `Weak<App>` — a strong handle cannot cross
//! into the event-loop closure — and hands the work to a function in the parent
//! module. Nothing here holds state of its own.

use super::*;

pub(crate) fn register_callbacks(ui: &App) {
    // The title bar's newspaper button.
    {
        let weak = ui.as_weak();
        ui.on_open_news(move || {
            if let Some(ui) = weak.upgrade() {
                toggle(&ui);
            }
        });
    }

    let state = ui.global::<NewsState>();
    {
        let weak = ui.as_weak();
        state.on_close(move || {
            if let Some(ui) = weak.upgrade() {
                close(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_close_detail(move || {
            if let Some(ui) = weak.upgrade() {
                close_detail(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_open_card(move |id| {
            if let Some(ui) = weak.upgrade() {
                open_card(&ui, id.as_str());
            }
        });
    }
    state.on_open_url(|url| {
        open_url(url.as_str());
    });
    {
        let weak = ui.as_weak();
        state.on_list_resized(move |width, height| {
            if let Some(ui) = weak.upgrade() {
                list_resized(&ui, width, height);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_list_scrolled(move |offset, viewport| {
            if let Some(ui) = weak.upgrade() {
                list_scrolled(&ui, offset, viewport);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_body_resized(move |width| {
            if let Some(ui) = weak.upgrade() {
                resize_body(&ui, width);
            }
        });
    }

    let search = ui.global::<NewsSearch>();
    {
        let weak = ui.as_weak();
        search.on_search(move || {
            if let Some(ui) = weak.upgrade() {
                search_now(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        search.on_chip_clicked(move |row, value| {
            if let Some(ui) = weak.upgrade() {
                chip_clicked(&ui, row.as_str(), value.as_str());
            }
        });
    }
    {
        let weak = ui.as_weak();
        search.on_chip_measured(move |row, value, width| {
            if let Some(ui) = weak.upgrade() {
                chip_measured(&ui, row.as_str(), value.as_str(), width);
            }
        });
    }
}
