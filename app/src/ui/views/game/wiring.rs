// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's callbacks, registered on `GameState`.

use super::*;

pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    let controller = Rc::new(RefCell::new(GameController::new(config)));
    // The list model is handed to the view once and then kept in sync in place by
    // `sync_rows`, so the rows the view renders are never recreated.
    ui.global::<GameState>()
        .set_rows(ModelRc::new(controller.borrow().rows_model.clone()));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&controller)));
    // The launch/download button mirrors the task centre; `project` runs once now
    // for the first paint and again after every task event.
    install::watch();
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
        // A reload that also moves the selection, for an instance that is only
        // in the *new* listing (see `GameController::reload_selecting`).
        let weak = ui.as_weak();
        state.on_refresh_selecting(move |id| {
            if let Some(ui) = weak.upgrade() {
                GameController::reload_selecting(&ui, Some(id.to_string()));
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
        state.on_move_selection(move |direction| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut controller = controller.borrow_mut();
            if !controller.move_selection(direction) {
                return;
            }
            controller.apply(&ui);
            // The list watches this serial and centres the new current row.
            let state = ui.global::<GameState>();
            state.set_selection_serial(state.get_selection_serial() + 1);
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
            if expanded {
                controller.expanded.insert(key.clone(), false);
            } else {
                // Accordion: opening a group closes every other one. The list
                // is then a sequence of one open page between collapsed headers,
                // which is what keeps the arrow/Alt-wheel step unambiguous.
                let keys: Vec<String> = controller
                    .groups
                    .iter()
                    .map(|group| group.key.clone())
                    .collect();
                for group in keys {
                    controller.expanded.insert(group, false);
                }
                controller.expanded.insert(key.clone(), true);
            }
            controller.persist();
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
                if !expanded {
                    // The group name is now a row; bring it under the floating
                    // sort/group bar so the newly opened group starts in view.
                    if let Some(y) = controller.group_header_y(&key) {
                        let state = ui.global::<GameState>();
                        state.set_group_scroll_y(y);
                        state.set_group_scroll_serial(state.get_group_scroll_serial() + 1);
                    }
                }
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
            if let Err(error) = config::save_config(&config_rc.borrow()) {
                log::warn!("failed to save the config: {error}");
            }
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        // `request` opens the page and lets it wait for the centre; the button
        // does not get to decide when a launch may start. The id is the
        // caller's, so the right-click menu can launch a card that is not the
        // current instance.
        let weak = ui.as_weak();
        state.on_launch(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let id = id.to_string();
            if id.is_empty() {
                return;
            }
            crate::ui::views::launch::request(&ui, id);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_start_download(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let id = id.to_string();
            if id.is_empty() {
                return;
            }
            install::start(&ui, id);
        });
    }
    {
        // One button stops whatever this instance is doing. The centre applies
        // the per-kind policy: a launch stops at once, an install asks after
        // ten seconds, and a queued task never asks at all.
        let weak = ui.as_weak();
        state.on_cancel_task(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            if id >= 0 {
                ui.global::<TaskState>().invoke_request_cancel(id);
            }
        });
    }
    {
        // The running game's Stop, which only the warning dialog can reach. The
        // id is the dialog's target rather than the selection, because the
        // right-click menu stops the card it was opened on.
        let weak = ui.as_weak();
        state.on_stop_game(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let dialogs = ui.global::<Dialogs>();
            dialogs.set_confirm_stop_game_visible(false);
            dialogs.set_stop_game_id("".into());
            let id = id.to_string();
            if id.is_empty() {
                return;
            }
            // The session watcher re-projects the button when the process is
            // really gone, so a stop that found nothing leaves it on Stop.
            if launch::running::terminate_all(&id) {
                log::info!("the player stopped the running instance '{id}'");
            } else {
                log::debug!("instance '{id}' had no running session to stop");
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_open_instance_menu(move |id, x, y| {
            let Some(ui) = weak.upgrade() else { return };
            let id = id.to_string();
            let Some((installed, starred)) = controller
                .borrow()
                .instances
                .iter()
                .find(|instance| instance.id == id)
                .map(|instance| (instance.installed, instance.is_starred()))
            else {
                return;
            };
            let (task, task_id) = install::task_of(&id);
            let menu = ui.global::<InstanceMenu>();
            menu.set_target(id.clone().into());
            menu.set_installed(installed);
            // A launch is not a task, so the first item needs the session
            // registry too: while a game runs it says Stop, not Launch.
            menu.set_running(launch::running::is_running(&id));
            menu.set_starred(starred);
            menu.set_task(task.into());
            menu.set_task_id(task_id);
            menu.set_anchor_x(x);
            menu.set_anchor_y(y);
            menu.set_visible(true);
        });
    }
    {
        state.on_open_instance_folder(|id| {
            let path = storage::LOCATIONS.instances.get_instance_root(&id);
            if let Err(error) =
                crate::ui::services::app_config::open_external(&path.to_string_lossy())
            {
                log::warn!("failed to open instance folder: {error}");
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_toggle_starred(move |id| {
            let Some(instance) = controller
                .borrow()
                .instances
                .iter()
                .find(|instance| instance.id == id.as_str())
                .cloned()
            else {
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
            crate::support::runtime::spawn(async move {
                if let Err(error) = instance::update_instance(config, &id).await {
                    log::error!("failed to update instance: {error}");
                    return;
                }
                // The row is rebuilt from the listing, so re-read it rather than
                // patching the one row in place.
                crate::ui::services::report::report(&weak, move |ui| GameController::reload(&ui));
            });
        });
    }

    {
        // The summary's gear opens the instance settings overlay, whose own
        // script (`instance_settings.rs`) fills it.
        let weak = ui.as_weak();
        state.on_open_instance_settings(move || {
            if let Some(ui) = weak.upgrade() {
                crate::ui::overlays::instance_settings::open(&ui);
            }
        });
    }
    {
        // The right-click menu's repair, which is the instance settings' own
        // action; both go through `install::repair` so the lock handling and
        // the queued install cannot drift apart.
        let weak = ui.as_weak();
        state.on_repair_install(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let id = id.to_string();
            if id.is_empty() {
                return;
            }
            install::repair(&ui, id);
        });
    }
    {
        // The right-click menu's delete: the settings' confirm dialog, opened
        // without the settings overlay in the way.
        let weak = ui.as_weak();
        state.on_open_delete_instance(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let id = id.to_string();
            if id.is_empty() {
                return;
            }
            crate::ui::overlays::instance_settings::open_delete_for(&ui, id);
        });
    }
    // `open-content` and `open-packs` belong to the content overlays' script
    // (`content`), which registers them after this one — a Slint `on_*`
    // setter replaces the handler, so nothing is wired for them here.
    {
        let weak = ui.as_weak();
        state.on_open_add_account(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Dialogs>().set_account_add_visible(true);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_open_connect(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<MultiplayerState>().invoke_open();
            }
        });
    }
    {
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
                crate::ui::overlays::account_view::open(&ui);
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
    let image =
        crate::ui::components::account_avatar::account_head(account, size).unwrap_or_default();
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
    if let Err(error) = config::save_config(&config.borrow()) {
        log::warn!("failed to save the config: {error}");
    }
}
