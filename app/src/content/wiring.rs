// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Every content-overlay callback, registered on the `ContentState` /
//! `ContentSearch` / `GameState` globals.

use super::*;

/// Registers every content-overlay callback on `ContentState` / `ContentSearch`
/// and takes over `GameState.open-content`.
pub fn setup(ui: &App) {
    push_grid_models(ui);
    setup_open_content(ui);
    setup_open_packs(ui);
    setup_source_switch(ui);
    setup_close(ui);

    setup_card_selection(ui);
    setup_detail_body(ui);
    setup_save_deletion(ui);
    setup_favorite_toggle(ui);

    setup_search_form(ui);
    setup_version_carousel(ui);
    setup_saves_and_screenshots(ui);
    setup_detail_actions(ui);
    setup_grid_resize(ui);

    load_favorites(ui);
}

/// The grids are pushed into the globals once.
///
/// The controller keeps the `VecModel` handles so a relayout can rewrite a row
/// in place; replacing the model instead would destroy and re-create every card
/// element, losing its hover and its decoded icon.
pub(crate) fn push_grid_models(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let ui_state = ui.global::<ContentState>();
    ui_state.set_saves(ModelRc::from(Rc::clone(&state.saves)));
    ui_state.set_local_mods(ModelRc::from(Rc::clone(&state.local_mods)));
    ui_state.set_local_resourcepacks(ModelRc::from(Rc::clone(&state.local_resourcepacks)));
    ui_state.set_remote_cards(ModelRc::from(Rc::clone(&state.remote)));
}

/// `GameState.open-content(kind)` — the instance summary's preview rows.
pub(crate) fn setup_open_content(ui: &App) {
    let weak = ui.as_weak();
    ui.global::<GameState>().on_open_content(move |kind| {
        let Some(ui) = weak.upgrade() else { return };
        let kind = kind.to_string();
        {
            let state = controller();
            let mut state = state.borrow_mut();
            state.sync_instance(&ui);
            state.kind = RemoteKind::from_key(&kind);
            state.source = "local".into();
            state.form = SearchForm::default();
            state.targets.clear();
            if kind == "resourcepacks" {
                // The list the row opened carries its kind; the packs panel
                // is only reachable from the footer.
                state.kind = RemoteKind::ResourcePacks;
            }
        }
        ui.global::<ContentState>()
            .set_open_panel(SharedString::from(kind.as_str()));
        ui.global::<ContentState>()
            .set_source(SharedString::from("local"));
        // The pagination bar is drawn from the list's own `totalPages`, so
        // opening a panel shows whatever that list had — nothing, the first
        // time.
        push_pages(&ui);
        match kind.as_str() {
            "saves" => {
                show_grid(&ui, Grid::Saves);
                load_saves(&ui);
            }
            "screenshots" => load_screenshots(&ui),
            "resourcepacks" => {
                show_grid(&ui, Grid::LocalResourcePacks);
                load_local_resourcepacks(&ui);
            }
            "mods" => {
                show_grid(&ui, Grid::LocalMods);
                load_local_mods(&ui);
            }
            _ => {}
        }
    });
}

/// `GameState.open-packs` — the footer's "install pack" button.
pub(crate) fn setup_open_packs(ui: &App) {
    let weak = ui.as_weak();
    ui.global::<GameState>().on_open_packs(move || {
        let Some(ui) = weak.upgrade() else { return };
        let instance_id = {
            let state = controller();
            let mut state = state.borrow_mut();
            state.sync_instance(&ui);
            state.kind = RemoteKind::Packs;
            state.platform = Platform::Modrinth;
            state.source = "modrinth".into();
            state.form = SearchForm::default();
            state.instance_id.clone()
        };
        // The lists are seeded from the instance's own loader and version, so
        // `instance.toml` is read on the runtime before the panel is filled in.
        let weak = weak.clone();
        crate::runtime::spawn(async move {
            let runtime = instance_runtime(&instance_id).await;
            crate::report::report(&weak, move |ui| {
                {
                    let state = controller();
                    let mut state = state.borrow_mut();
                    let list = list_key(state.kind, state.platform.key());
                    initialize_list(&mut state, &list, &runtime);
                    state.targets.clear();
                }
                let ui_state = ui.global::<ContentState>();
                ui_state.set_open_panel(SharedString::from("packs"));
                ui_state.set_source(SharedString::from("modrinth"));
                ui_state.set_remote_kind(SharedString::from(RemoteKind::Packs.key()));
                ensure_version_options(&ui);
                push_search(&ui);
                show_grid(&ui, Grid::Remote);
                run_search(&ui, 1);
            });
        });
    });
}

/// The source switcher (Local / Modrinth / CurseForge).
pub(crate) fn setup_source_switch(ui: &App) {
    let weak = ui.as_weak();
    ui.global::<ContentState>().on_set_source(move |source| {
        let Some(ui) = weak.upgrade() else { return };
        let source = source.to_string();
        {
            let state = controller();
            let mut state = state.borrow_mut();
            state.sync_instance(&ui);
            state.source = source.clone();
            if source != "local" {
                state.platform = if source == "modrinth" {
                    Platform::Modrinth
                } else {
                    Platform::CurseForge
                };
                // A source switch resets the query, the page and the
                // selections — the filters it seeds come back only if this list
                // has not already been opened on this instance.
                state.form = SearchForm::default();
            }
        }
        // The remote lists are seeded from the instance's own loader and
        // version, so `instance.toml` is read on the runtime before the
        // panel is filled in. A local list has nothing to seed.
        let instance_id = controller().borrow().instance_id.clone();
        let source = source.clone();
        let weak = weak.clone();
        crate::runtime::spawn(async move {
            let runtime = instance_runtime(&instance_id).await;
            crate::report::report(&weak, move |ui| {
                if source != "local" {
                    let state = controller();
                    let mut state = state.borrow_mut();
                    let list = list_key(state.kind, state.platform.key());
                    initialize_list(&mut state, &list, &runtime);
                    state.targets.clear();
                }
                ui.global::<ContentState>()
                    .set_source(SharedString::from(source.as_str()));
                // The list being shown is the grid a resize re-lays out from
                // here on, and it is re-laid out now — the local list in
                // particular was loaded while another source was open, at
                // whatever width the panel had then.
                show_grid(
                    &ui,
                    if source == "local" {
                        if controller().borrow().kind == RemoteKind::Mods {
                            Grid::LocalMods
                        } else {
                            Grid::LocalResourcePacks
                        }
                    } else {
                        Grid::Remote
                    },
                );
                // A source switch shows the other list's own page state, which
                // for a list that has not searched yet is a hidden bar.
                push_pages(&ui);
                match source.as_str() {
                    "modrinth" | "curseforge" => {
                        ensure_version_options(&ui);
                        push_search(&ui);
                        run_search(&ui, 1);
                    }
                    _ => {
                        let kind = controller().borrow().kind;
                        match kind {
                            RemoteKind::Mods => load_local_mods(&ui),
                            RemoteKind::ResourcePacks => load_local_resourcepacks(&ui),
                            RemoteKind::Packs => {}
                        }
                    }
                }
            });
        });
    });
}

/// Closing the panel, and closing a detail panel back to its list.
pub(crate) fn setup_close(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_close(move || {
            let Some(ui) = weak.upgrade() else { return };
            let ui_state = ui.global::<ContentState>();
            ui_state.set_open_panel(SharedString::default());
            ui_state.set_open_detail(SharedString::default());
            ui_state.set_screenshot_index(0);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_close_detail(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<ContentState>()
                .set_open_detail(SharedString::default());
            controller().borrow_mut().detail = None;
        });
    }
}

/// A remote card opens its detail panel, and a card's folder button reveals it.
/// A local card has no detail view and carries only the folder and delete
/// buttons.
pub(crate) fn setup_card_selection(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_select_card(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let id = id.to_string();
            let Some(CardTarget::Remote { platform, id }) =
                controller().borrow().targets.get(&id).cloned()
            else {
                return;
            };
            open_detail(&ui, platform, id);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_open_path(move |id| {
            let Some(_ui) = weak.upgrade() else { return };
            let target = controller().borrow().targets.get(id.as_str()).cloned();
            let Some(path) = (match target {
                Some(CardTarget::Path(path)) | Some(CardTarget::Save(path)) => Some(path),
                _ => None,
            }) else {
                return;
            };
            // The file manager opens with the file selected, which
            // `open_external` would not do — it launches it.
            if let Err(error) = crate::config_bridge::reveal_in_dir(&path) {
                log::warn!("failed to reveal {path}: {error}");
            }
        });
    }
}

/// The detail panel's body: its width, its collapsible sections and its links.
pub(crate) fn setup_detail_body(ui: &App) {
    {
        // The view reports the width it was given rather than waiting to be
        // asked, because a document is a function of its width and a width
        // nobody is told about is a document that never re-wraps.
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_body_resized(move |width| {
            let Some(ui) = weak.upgrade() else { return };
            layout_detail_body(&ui, width);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>()
            .on_body_section_toggled(move |id| {
                let Some(ui) = weak.upgrade() else { return };
                toggle_detail_section(&ui, id as u32);
            });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_open_url(move |url| {
            let Some(_ui) = weak.upgrade() else { return };
            if let Err(error) = crate::config_bridge::open_external(&url) {
                log::warn!("failed to open {url}: {error}");
            }
        });
    }
}

/// Deleting a save: the direct action and the confirmation dialog that guards
/// it (`ConfirmDeleteSave`, so a mis-click cannot lose a world).
pub(crate) fn setup_save_deletion(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_delete_save(move |folder| {
            let Some(ui) = weak.upgrade() else { return };
            let folder = folder.to_string();
            let instance = controller().borrow().instance_id.clone();
            // The event loop is reached through a weak handle: a strong `App`
            // may not cross into the spawned task.
            let weak = ui.as_weak();
            crate::runtime::spawn(async move {
                if let Err(error) = content::saves::delete_save(&instance, &folder).await {
                    log::error!("failed to delete the save {folder}: {error}");
                    return;
                }
                crate::report::report(&weak, move |ui| load_saves(&ui));
            });
        });
    }
    {
        // The saves grid's trash button. It opens `ConfirmDeleteSave` rather than
        // deleting — the dialog is the only thing standing between a mis-click
        // and a lost world.
        let weak = ui.as_weak();
        ui.global::<ContentState>()
            .on_request_delete_save(move |folder, level_name| {
                let Some(ui) = weak.upgrade() else { return };
                {
                    let state = ui.global::<DeleteSaveState>();
                    state.set_folder(folder);
                    state.set_level_name(level_name);
                    // A delete cannot be left half-done from a previous one.
                    state.set_deleting(false);
                }
                ui.global::<Dialogs>().set_confirm_delete_save_visible(true);
            });
    }
    {
        let weak = ui.as_weak();
        ui.global::<DeleteSaveState>().on_cancel(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<Dialogs>()
                .set_confirm_delete_save_visible(false);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<DeleteSaveState>().on_confirm(move || {
            let Some(ui) = weak.upgrade() else { return };
            let folder = ui.global::<DeleteSaveState>().get_folder();
            if folder.is_empty() {
                return;
            }
            ui.global::<DeleteSaveState>().set_deleting(true);
            let instance = controller().borrow().instance_id.clone();
            // The event loop is reached through a weak handle: a strong `App`
            // may not cross into the spawned task.
            let weak = ui.as_weak();
            crate::runtime::spawn(async move {
                let result = content::saves::delete_save(&instance, &folder).await;
                crate::report::report(&weak, move |ui| {
                    ui.global::<DeleteSaveState>().set_deleting(false);
                    // A failure is logged and the dialog left open, so a save
                    // that could not be removed is still there to be retried.
                    if let Err(error) = result {
                        log::error!("failed to delete the save {folder}: {error}");
                        return;
                    }
                    ui.global::<Dialogs>()
                        .set_confirm_delete_save_visible(false);
                    load_saves(&ui);
                });
            });
        });
    }
}

/// Favourites. Updated optimistically and rolled back if the write fails.
pub(crate) fn setup_favorite_toggle(ui: &App) {
    let weak = ui.as_weak();
    ui.global::<ContentState>().on_toggle_favorite(move |id| {
        let Some(ui) = weak.upgrade() else { return };
        let Some(CardTarget::Remote { platform, id }) =
            controller().borrow().targets.get(id.as_str()).cloned()
        else {
            return;
        };
        let kind = controller().borrow().kind.key();
        let key = favorite_key(platform.key(), kind, &id);
        let added = {
            let state = controller();
            let mut state = state.borrow_mut();
            let added = !state.favorites.contains(&key);
            if added {
                state.favorites.insert(key.clone());
            } else {
                state.favorites.remove(&key);
            }
            added
        };
        let write = if added {
            content::favorites::add_favorite(platform.key().into(), kind.into(), id)
        } else {
            content::favorites::remove_favorite(platform.key().into(), kind.into(), id)
        };
        if let Err(error) = write {
            log::error!("failed to write favorites: {error}");
            let state = controller();
            let mut state = state.borrow_mut();
            if added {
                state.favorites.remove(&key);
            } else {
                state.favorites.insert(key);
            }
        }
        refresh_favorite_flags(&ui);
    });
}

/// The search form: the query, the filter chips and the pagination.
pub(crate) fn setup_search_form(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentSearch>().on_search(move || {
            let Some(ui) = weak.upgrade() else { return };
            controller().borrow_mut().form.query =
                ui.global::<ContentSearch>().get_query().to_string();
            run_search(&ui, 1);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentSearch>().on_chip_clicked(move |value| {
            let Some(ui) = weak.upgrade() else { return };
            toggle_chip(&ui, value.as_ref());
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentSearch>().on_go_to_page(move |page| {
            let Some(ui) = weak.upgrade() else { return };
            // With no result yet the count is 0, so every page is out of range
            // and nothing is requested.
            let (_, total_pages) = {
                let state = controller();
                let state = state.borrow();
                list_pages(&state)
            };
            let page = page.max(1) as usize;
            if page > total_pages {
                return;
            }
            run_search(&ui, page);
        });
    }
}

/// The version carousel: chip widths are what its offset is summed from, and
/// the pagers step it. An index would land the other rows' widths on the
/// version slots, so every chip reports its value.
pub(crate) fn setup_version_carousel(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentSearch>()
            .on_chip_measured(move |value, width| {
                let Some(ui) = weak.upgrade() else { return };
                let (moved, recorded) = {
                    let state = controller();
                    let mut state = state.borrow_mut();
                    // A version chip moves the carousel's track; every other
                    // chip is one of the wrapping rows, which are laid out from
                    // these widths.
                    let moved = match state
                        .version_options
                        .iter()
                        .position(|option| option == value.as_str())
                    {
                        Some(index) => {
                            if state.version_widths.len() <= index {
                                state.version_widths.resize(index + 1, 0.0);
                            }
                            let previous = state.version_widths[index];
                            state.version_widths[index] = width;
                            // Only a chip inside the current page's prefix
                            // moves the track; a later one just has its width
                            // recorded.
                            previous != width
                                && index < (state.version_page + 1) * VERSIONS_PER_PAGE
                        }
                        None => false,
                    };
                    let recorded =
                        state.filter_widths.insert(value.to_string(), width) != Some(width);
                    (moved, recorded)
                };
                if moved {
                    let state = controller();
                    let state = state.borrow();
                    ui.global::<ContentSearch>()
                        .set_version_offset(version_offset(&state));
                }
                if recorded {
                    relayout_filters(&ui);
                }
            });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentSearch>().on_version_page_prev(move || {
            let Some(ui) = weak.upgrade() else { return };
            {
                let state = controller();
                let mut state = state.borrow_mut();
                state.version_page = state.version_page.saturating_sub(1);
            }
            push_version(&ui);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentSearch>().on_version_page_next(move || {
            let Some(ui) = weak.upgrade() else { return };
            {
                let state = controller();
                let mut state = state.borrow_mut();
                let last = version_page_count(&state).saturating_sub(1);
                state.version_page = (state.version_page + 1).min(last);
            }
            push_version(&ui);
        });
    }
}

/// The saves' expansion and the screenshot viewer.
pub(crate) fn setup_saves_and_screenshots(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_select_save(move |folder| {
            let Some(ui) = weak.upgrade() else { return };
            select_save(&ui, folder.as_ref());
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_set_screenshot(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            set_screenshot(&ui, index);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_screenshot_next(move || {
            let Some(ui) = weak.upgrade() else { return };
            let index = ui.global::<ContentState>().get_screenshot_index() + 1;
            set_screenshot(&ui, index);
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_screenshot_prev(move || {
            let Some(ui) = weak.upgrade() else { return };
            let index = ui.global::<ContentState>().get_screenshot_index() - 1;
            set_screenshot(&ui, index);
        });
    }
}

/// Reads the stored favourites into the controller once at startup, since every
/// panel shares them.
pub(crate) fn load_favorites(ui: &App) {
    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let favorites = content::favorites::list_favorites().unwrap_or_default();
        let keys: HashSet<String> = favorites
            .into_iter()
            .map(|favorite| {
                favorite_key(
                    &favorite.platform,
                    &favorite.content_type,
                    &favorite.project_id,
                )
            })
            .collect();
        crate::report::report(&weak, move |ui| {
            controller().borrow_mut().favorites = keys;
            refresh_favorite_flags(&ui);
        });
    });
}

/// The detail panel's Download and Remove buttons.
pub(crate) fn setup_detail_actions(ui: &App) {
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_download(move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(detail) = controller().borrow().detail.clone() else {
                return;
            };
            let instance = controller().borrow().instance_id.clone();
            ui.global::<ContentState>().set_detail_operating(true);
            let token = controller().borrow_mut().detail_download_gate.issue();
            let weak = ui.as_weak();
            crate::runtime::spawn(async move {
                let outcome = install(&instance, &detail, weak.clone(), token.clone()).await;
                deliver(&weak, &token, move |ui| {
                    let state = ui.global::<ContentState>();
                    state.set_detail_operating(false);
                    state.set_detail_progress(0.0);
                    state.set_detail_progress_max(0.0);
                    state.set_detail_progress_text("".into());
                    match outcome {
                        Ok(()) => {
                            refresh_installed(&ui);
                            // The local list the mod landed in is stale now.
                            load_local_mods(&ui);
                        }
                        Err(error) => log::error!("failed to download content: {error}"),
                    }
                });
            });
        });
    }
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_remove(move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(detail) = controller().borrow().detail.clone() else {
                return;
            };
            ui.global::<ContentState>().set_detail_operating(true);
            let files: Vec<String> = detail
                .installed_mods
                .iter()
                .map(|mod_info| mod_info.path.to_string_lossy().to_string())
                .collect();
            let instance = controller().borrow().instance_id.clone();
            match content::mods::remote::remove_mod_files(&instance, files) {
                Ok(()) => {
                    refresh_installed(&ui);
                    load_local_mods(&ui);
                }
                Err(error) => log::error!("failed to remove content: {error}"),
            }
            ui.global::<ContentState>().set_detail_operating(false);
        });
    }
}

/// A grid reports the width it has to lay out in, so the open panel re-lays
/// itself out when the window resizes.
pub(crate) fn setup_grid_resize(ui: &App) {
    let weak = ui.as_weak();
    ui.global::<ContentState>()
        .on_grid_resized(move |width, height| {
            let Some(ui) = weak.upgrade() else { return };
            {
                let state = controller();
                let mut state = state.borrow_mut();
                state.grid_width = width as i32;
                state.panel_height = height as i32;
            }
            // The search panel's rows wrap at the panel's width, so a resize
            // changes how many lines each of them takes — and with it the
            // panel's height. The row heights were computed for the old width,
            // and the rows then sat too close together (or too far apart) until
            // something else re-measured them.
            relayout_filters(&ui);
            relayout(&ui);
        });
}

pub(crate) fn favorite_key(platform: &str, kind: &str, id: &str) -> String {
    format!("{platform}:{kind}:{id}")
}

/// The instance's loader and Minecraft version, read off `instance.toml`.
///
/// The remote lists are seeded from it, so it is fetched on the runtime whenever
/// a list opens rather than on the UI thread. An instance that cannot be read
/// seeds nothing, which is the empty-loader/version key the seeding below
/// compares against.
pub(crate) async fn instance_runtime(instance_id: &str) -> InstanceRuntime {
    instance::get_instance_by_id(instance_id)
        .await
        .map(|instance| instance.config.runtime)
        .unwrap_or_default()
}

/// The loader and version the list opens on, seeded from the instance's
/// runtime. It happens once per instance per list, so a list that is switched
/// away from and back is *not* re-seeded.
pub(crate) fn initialize_list(
    state: &mut ContentController,
    list: &str,
    runtime: &InstanceRuntime,
) {
    let key = format!(
        "{}|{}",
        runtime
            .mod_loader_type
            .as_ref()
            .map(|loader| loader.to_string())
            .unwrap_or_default(),
        runtime.minecraft
    );
    let entry = state.lists.entry(list.to_string()).or_default();
    if entry.initialized_for.as_deref() == Some(key.as_str()) {
        return;
    }
    entry.initialized_for = Some(key);
    seed_filters(state, runtime);
}

pub(crate) fn seed_filters(state: &mut ContentController, runtime: &InstanceRuntime) {
    if state.kind.has_loaders()
        && let Some(loader) = &runtime.mod_loader_type
    {
        // `ModLoaderType` displays as "Fabric"/"NeoForge"; the API and the
        // filter chips use the lower-case key.
        state.form.loaders.push(loader.to_string().to_lowercase());
    }
    if !runtime.minecraft.is_empty() {
        state.form.versions.push(runtime.minecraft.clone());
    }
}
