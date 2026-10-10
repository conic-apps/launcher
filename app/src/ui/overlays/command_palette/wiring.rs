// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The command-palette callbacks, registered on `CommandPaletteState`.

use super::*;

/// Registers every command-palette callback on `CommandPaletteState`.
///
/// The two openers are wired in the view, the way the title bar's other actions
/// are: `CommandPaletteState.open()` for the search field's click and
/// `CommandPaletteState.toggle()` for the `Ctrl`/`⌘` + `/` shortcut, which the
/// app-level key scope in `app.slint` binds.
pub fn setup(ui: &App) {
    setup_visibility(ui);
    setup_selection(ui);
    setup_search(ui);
}

pub(crate) fn setup_visibility(ui: &App) {
    let controller = controller();
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_toggle(move || {
            let Some(ui) = weak.upgrade() else { return };
            controller.borrow_mut().toggle(&ui);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_open(move || {
            let Some(ui) = weak.upgrade() else { return };
            if ui.global::<CommandPaletteState>().get_visible() {
                return;
            }
            PaletteController::open(&ui);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_close(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<CommandPaletteState>().set_visible(false);
            }
        });
    }
}

pub(crate) fn setup_selection(ui: &App) {
    let controller = controller();
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_click(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            controller.borrow_mut().click(&ui, index.max(0) as usize);
        });
    }
    // Enter, which acts on the selection rather than on a row.
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>()
            .on_perform_selected(move || {
                let Some(ui) = weak.upgrade() else { return };
                let selected = controller.borrow().selected;
                controller.borrow_mut().perform(&ui, selected);
            });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        // The pointer marks a row; it does not choose it.
        ui.global::<CommandPaletteState>().on_hover(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            let index = if index < 0 {
                None
            } else {
                Some(index as usize)
            };
            controller.borrow_mut().hover(&ui, index);
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>()
            .on_move_selection(move |delta| {
                let Some(ui) = weak.upgrade() else { return };
                controller.borrow_mut().move_selection(&ui, delta);
            });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>()
            .on_query_changed(move || {
                let Some(ui) = weak.upgrade() else { return };
                controller.borrow_mut().query_changed(&ui);
            });
    }
    {
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_clear_query(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<CommandPaletteState>()
                .set_query(SharedString::new());
            CONTROLLER.with(|controller| controller.borrow_mut().query_changed(&ui));
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>()
            .on_enter_mode(move |kind, source| {
                let Some(ui) = weak.upgrade() else { return };
                let (kind, source) = (kind.to_string(), source.to_string());
                controller
                    .borrow_mut()
                    .enter_mode(&ui, kind.as_str(), source.as_str());
            });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_back_to_root(move || {
            let Some(ui) = weak.upgrade() else { return };
            controller.borrow_mut().back_to_root(&ui);
        });
    }
}

/// The 250ms debounce elapsed, so the search itself runs.
pub(crate) fn setup_search(ui: &App) {
    let controller = controller();
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<CommandPaletteState>().on_run_search(move || {
            let Some(ui) = weak.upgrade() else { return };
            controller.borrow_mut().run_search(&ui);
        });
    }
}
