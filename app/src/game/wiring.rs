// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's callbacks, registered on `GameState`.

use super::*;

/// Registers every game view callback on the `GameState` global.
pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    let controller = Rc::new(RefCell::new(GameController::new(config)));
    // The list model is handed to the view once and then kept in sync in place by
    // `sync_rows`, so the rows the view renders are never recreated.
    ui.global::<GameState>()
        .set_rows(ModelRc::new(controller.borrow().rows_model.clone()));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&controller)));
    GameController::reload(ui);

    let state = ui.global::<GameState>();

    {
        let weak = ui.as_weak();
        state.on_refresh(move || {
            if let Some(ui) = weak.upgrade() {
                GameController::reload(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_select_instance(move |id| {
            let id = id.to_string();
            if controller.borrow().current_id.as_deref() == Some(id.as_str()) {
                return;
            }
            controller.borrow_mut().current_id = Some(id);
            controller.borrow().persist();
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_toggle_group(move |key| {
            let key = key.to_string();
            let mut controller = controller.borrow_mut();
            let expanded = controller.expanded(&key);
            // Collapsing closes the gap on the 300ms collapse curve; re-opening
            // is a FLIP, like every other layout change. `expanded` is the state
            // before the toggle, so the curve is keyed off its negation.
            controller.flip = !expanded;
            controller.expanded.insert(key, !expanded);
            controller.persist();
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_set_sort(move |mode| {
            let sort = match mode.as_str() {
                "name" => SortBy::Name,
                "version" => SortBy::Version,
                "lastplay" => SortBy::LastPlayed,
                _ => SortBy::Playtime,
            };
            // The sort is read by the listing, so it has to be in place before
            // `reload` spawns; the rows follow when the listing lands.
            controller.borrow_mut().sort = sort;
            controller.borrow().persist();
            if let Some(ui) = weak.upgrade() {
                GameController::reload(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_set_group_mode(move |mode| {
            let mut controller = controller.borrow_mut();
            controller.group_mode = if mode.as_str() == "loader" {
                "loader"
            } else {
                "none"
            };
            controller.persist();
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_search_changed(move |query| {
            controller.borrow_mut().search = query.to_string();
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_select_account(move |key| {
            let key = key.to_string();
            let account = controller
                .borrow()
                .accounts
                .iter()
                .find(|account| account.key() == key)
                .cloned();
            let Some(account) = account else { return };
            let config_rc = controller.borrow().config.clone();
            config_rc.borrow_mut().current_account = Some(account);
            let _ = config::save_config(&config_rc.borrow());
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_launch(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Navigation>().invoke_navigate("launch".into());
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_repair_and_launch(move || {
            let Some(id) = controller.borrow().current_id.clone() else {
                return;
            };
            let weak = weak.clone();
            crate::runtime::spawn(async move {
                if let Err(error) = instance::remove_install_lock(&id).await {
                    log::error!("failed to remove install lock: {error}");
                    return;
                }
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    // The launch page re-runs the install flow when the lock is gone.
                    ui.global::<Navigation>().invoke_navigate("launch".into());
                });
            });
        });
    }
    {
        let controller = Rc::clone(&controller);
        state.on_open_instance_folder(move || {
            let Some(id) = controller.borrow().current_id.clone() else {
                return;
            };
            let path = folder::DATA_LOCATION.get_instance_root(&id);
            if let Err(error) = crate::config_bridge::open_external(&path.to_string_lossy()) {
                log::warn!("failed to open instance folder: {error}");
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_toggle_starred(move || {
            let Some(instance) = controller.borrow().current().cloned() else {
                return;
            };
            let mut config = instance.config.clone();
            let mut groups = config.group.clone().unwrap_or_default();
            if instance.is_starred() {
                groups.retain(|group| group != "starred");
            } else {
                groups.push("starred".to_string());
            }
            config.group = Some(groups);
            let id = instance.id;
            let weak = weak.clone();
            crate::runtime::spawn(async move {
                if let Err(error) = instance::update_instance(config, &id).await {
                    log::error!("failed to update instance: {error}");
                    return;
                }
                // The row is rebuilt from the listing, so re-read it rather than
                // patching the one row in place.
                let _ = weak.upgrade_in_event_loop(move |ui| GameController::reload(&ui));
            });
        });
    }

    {
        // The summary's gear opens the instance settings overlay, whose own
        // script (`instance_settings.rs`) fills it.
        let weak = ui.as_weak();
        state.on_open_instance_settings(move || {
            if let Some(ui) = weak.upgrade() {
                crate::instance_settings::open(&ui);
            }
        });
    }
    // `open-content` and `open-packs` belong to the content overlays' script
    // (`content`), which registers them after this one — a Slint `on_*`
    // setter replaces the handler, so nothing is wired for them here.
    {
        // The footer's "+" avatar and its "not logged in" label open the
        // add-account dialog.
        let weak = ui.as_weak();
        state.on_open_add_account(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Dialogs>().set_account_add_visible(true);
            }
        });
    }
    {
        // The footer's globe opens the multiplayer dialog: it checks the Conic
        // Nexus library and shows either the download screen or the manager.
        let weak = ui.as_weak();
        state.on_open_connect(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<MultiplayerState>().invoke_open();
            }
        });
    }
    {
        // The footer's "New instance" opens the create-instance dialog.
        let weak = ui.as_weak();
        state.on_new_instance(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Dialogs>().set_create_instance_visible(true);
            }
        });
    }
    {
        // The add-account dialog runs this after every successful add; it is
        // never run at startup.
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_select_first_account(move || {
            let config = Rc::clone(&controller.borrow().config);
            select_first_account_if_none(&config);
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    state.on_debug_log(|message| log::info!(target: "probe", "{message}"));
    {
        let weak = ui.as_weak();
        state.on_open_accounts(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Navigation>().invoke_navigate("accounts".into());
            }
        });
    }
}

/// The head an account's avatar shows, memoised by `<key>@<size>`.
///
/// `None` is the logged-out footer, which shows the bundled wide Steve.
pub(crate) fn account_avatar(
    account: Option<&Account>,
    size: u32,
    cache: &mut HashMap<String, Image>,
) -> Image {
    let key = match account {
        Some(account) => format!("{}@{size}", account.key()),
        None => format!("steve@{size}"),
    };
    if let Some(image) = cache.get(&key) {
        return image.clone();
    }
    // A skin that cannot be decoded (a URL the crate failed to download) leaves
    // the placeholder disc up.
    let image = crate::account_avatar::account_head(account, size).unwrap_or_default();
    cache.insert(key, image.clone());
    image
}

/// Picks an account when none is selected, after an account is added.
///
/// The precedence is Microsoft, then offline, then Yggdrasil — note that it is
/// *not* the order `reload_accounts` pushes them into the footer's list in.
pub fn select_first_account_if_none(config: &Rc<RefCell<config::Config>>) {
    if config.borrow().current_account.is_some() {
        return;
    }
    let accounts = account::list_accounts();
    let selected = accounts
        .microsoft
        .first()
        .cloned()
        .map(Account::Microsoft)
        .or_else(|| accounts.offline.first().cloned().map(Account::Offline))
        .or_else(|| accounts.yggdrasil.first().cloned().map(Account::Yggdrasil));
    config.borrow_mut().current_account = selected;
    let _ = config::save_config(&config.borrow());
}
