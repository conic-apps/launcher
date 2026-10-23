// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app-wide Escape shortcut.
//!
//! Escape is not just "navigate to the game page": every transient surface
//! above it has to go too, or the user lands on the home page with a dialog or
//! overlay still covering it. This service is the one place that knows the
//! whole set: it closes each surface through its own close path (so pending
//! work is cancelled and models are cleared, not just hidden) and then asks
//! `Navigation` for the home page.
//!
//! The command palette consumes key events while it is open; its last Escape
//! step calls the same `Navigation.escape` callback, so a press from the
//! palette does the same thing.

use slint::ComponentHandle;

use crate::slint_backend::{
    AccountAddState, AccountViewState, App, CommandPaletteState, ContentState, DeleteAccountState,
    DeleteInstanceState, DeleteSaveState, Dialogs, Dropdown, FocusManager, InstanceSettingsState,
    ModpackImportState, MultiplayerState, MusicState, Navigation, NewsState, NotificationState,
};

/// Registers the Escape handler on the app.
pub(crate) fn setup(ui: &App) {
    let weak = ui.as_weak();
    ui.global::<Navigation>().on_escape(move || {
        let Some(ui) = weak.upgrade() else { return };
        let page = ui.global::<Navigation>().get_current_page();
        close(&ui);
        ui.global::<Navigation>().invoke_navigate("game".into());
        log::debug!(target: "shell", "escape: {page} -> game");
    });
}

fn close(ui: &App) {
    // Surfaces with a close path of their own. These clear models and cancel
    // work, so they are used instead of writing their visibility flags.
    ui.global::<ModpackImportState>().invoke_cancel();
    ui.global::<AccountAddState>().invoke_close();
    ui.global::<NewsState>().invoke_close();
    ui.global::<ContentState>().invoke_close();
    ui.global::<NotificationState>().invoke_close();
    // Previews are a second, independent layer: close the centre's cards and
    // drop the floating previews.
    ui.global::<NotificationState>().invoke_expire_all();
    ui.global::<AccountViewState>().invoke_close();
    ui.global::<InstanceSettingsState>().invoke_close();
    ui.global::<MultiplayerState>().invoke_close();
    ui.global::<MusicState>().invoke_close_panel();
    ui.global::<CommandPaletteState>().invoke_close();
    ui.global::<Dropdown>().set_open(false);
    ui.global::<FocusManager>().invoke_clear();

    // The delete dialogs cancel through their own callbacks; the rest only
    // have the visibility flag.
    ui.global::<DeleteInstanceState>().invoke_cancel();
    ui.global::<DeleteSaveState>().invoke_cancel();
    ui.global::<DeleteAccountState>().invoke_cancel();
    let dialogs = ui.global::<Dialogs>();
    dialogs.set_account_add_visible(false);
    dialogs.set_create_instance_visible(false);
    dialogs.set_no_account_error_visible(false);
    dialogs.set_no_microsoft_account_error_visible(false);
    dialogs.set_account_refresh_failed_visible(false);
    dialogs.set_no_suitable_java_error_visible(false);
    dialogs.set_confirm_delete_instance_visible(false);
    dialogs.set_confirm_delete_save_visible(false);
    dialogs.set_confirm_quit_app_visible(false);
    dialogs.set_restart_required_visible(false);
    dialogs.set_confirm_delete_account_visible(false);
}
