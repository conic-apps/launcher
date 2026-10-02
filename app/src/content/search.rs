// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The search panel: the version carousel and page bar, and the remote
//! listings for both platforms.

use super::*;

/// The Minecraft release list the version filter is built from, fetched once.
pub(crate) fn ensure_version_options(ui: &App) {
    if !controller().borrow().version_options.is_empty() {
        // Already fetched: the sync still has to run, because every open
        // re-syncs the carousel to the instance's own version.
        controller().borrow_mut().sync_version_page();
        return;
    }
    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let manifest = match install::get_minecraft_version_list().await {
            Ok(manifest) => manifest,
            Err(error) => {
                log::warn!(
                    "failed to fetch the Minecraft version list for the version filter: {error}"
                );
                return;
            }
        };
        let mut options: Vec<String> = manifest
            .versions
            .iter()
            .filter(|version| version.r#type == "release")
            .map(|version| version.id.clone())
            .collect();
        // The ids sort the same way as `releaseTime` descending for the release
        // line, and the list carries no 1.x variants that would differ.
        options.sort_by_key(|id| std::cmp::Reverse(version_order(id)));
        options.dedup();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                state.version_options = options;
                // The selection can only be found once this list exists, so the
                // sync waits for the manifest.
                state.sync_version_page();
            }
            push_search(&ui);
        });
    });
}

pub(crate) fn version_order(id: &str) -> (u32, u32, u32) {
    let mut parts = id.split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// The carousel's page count — how many times the track can step.
pub(crate) fn version_page_count(state: &ContentController) -> usize {
    state.version_options.len().div_ceil(VERSIONS_PER_PAGE)
}

/// The width of every chip before the page's first, each followed by the
/// track's 6px gap. The widths are the ones the chips reported through
/// `chip-measured`; a chip that has not been laid out yet counts as nothing,
/// and the offset is recomputed when it reports.
pub(crate) fn version_offset(state: &ContentController) -> f32 {
    let first = state.version_page * VERSIONS_PER_PAGE;
    (0..first.min(state.version_widths.len()))
        .map(|index| state.version_widths[index] + VERSION_GAP)
        .sum()
}

/// The key one list's page state is filed under: each kind and source pair is a
/// list of its own, with its own page numbers.
///
/// The key is the *source* (`"local" | "modrinth" | "curseforge"`), not the
/// platform: the local lists have no platform of their own, and keying them on
/// whichever one the last remote list happened to set made the local view read
/// that list's page count — the local mods list drew modrinth's 158 pages.
pub(crate) fn list_key(kind: RemoteKind, source: &str) -> String {
    format!("{}:{}", kind.key(), source)
}

/// That list's `(current page, total pages)` — `(1, 0)` for one that has not
/// searched yet, which is what hides its pagination bar.
pub(crate) fn list_pages(state: &ContentController) -> (usize, usize) {
    pages_of(&state.pages, state.kind, &state.source)
}

/// The lookup itself, apart from the controller so the per-list rule can be
/// tested without one.
pub(crate) fn pages_of(
    pages: &HashMap<String, (usize, usize)>,
    kind: RemoteKind,
    source: &str,
) -> (usize, usize) {
    pages
        .get(&list_key(kind, source))
        .copied()
        .unwrap_or((1, 0))
}

/// Pushes the open list's page, its page count and its page buttons.
pub(crate) fn push_pages(ui: &App) {
    let (page, total_pages) = {
        let state = controller();
        let state = state.borrow();
        list_pages(&state)
    };
    let search = ui.global::<ContentSearch>();
    search.set_current_page(page as i32);
    search.set_total_pages(total_pages as i32);
    search.set_pages(ModelRc::from(Rc::new(VecModel::from(pagination_pages(
        total_pages,
        page,
    )))));
}

/// A filter chip's height.
pub(crate) const FILTER_CHIP_HEIGHT: f32 = 20.0;
/// The gap between filter chips.
pub(crate) const FILTER_CHIP_GAP: f32 = 6.0;
/// The filter row's carousel: the 26px pager row.
pub(crate) const FILTER_CAROUSEL_HEIGHT: f32 = 26.0;

/// A row's label takes 52px and 10px of gap out of the panel's content box —
/// so the chips wrap inside `panel - 48 (padding) - 62 (label and gap)`.
///
/// Returns the row's height: 20px per line with the 6px gap between them.
pub(crate) fn filter_row_height(state: &ContentController, chips: &ModelRc<FilterChip>) -> f32 {
    let available = (state.grid_width - 48 - 62).max(0) as f32;
    let width = |chip: &FilterChip| {
        state
            .filter_widths
            .get(chip.value.as_str())
            .copied()
            .unwrap_or(0.0)
    };
    let widths = (0..chips.row_count())
        .filter_map(|index| chips.row_data(index))
        .map(|chip| width(&chip));
    filter_row_height_of(available, widths)
}

/// The wrapping itself, apart from the controller so that it can be tested: the
/// chips run left to right and a chip that does not fit opens a new line.
pub(crate) fn filter_row_height_of(available: f32, widths: impl Iterator<Item = f32>) -> f32 {
    let mut lines = 1.0f32;
    let mut x = 0.0f32;
    for width in widths {
        if x > 0.0 && x + FILTER_CHIP_GAP + width > available {
            lines += 1.0;
            x = width;
        } else {
            x += if x > 0.0 { FILTER_CHIP_GAP } else { 0.0 } + width;
        }
    }
    lines * FILTER_CHIP_HEIGHT + (lines - 1.0) * FILTER_CHIP_GAP
}

/// Recounts every row's height from the widths reported so far and rewrites the
/// rows that changed, in place.
pub(crate) fn relayout_filters(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let search = ui.global::<ContentSearch>();
    for index in 0..state.filter_rows.row_count() {
        let Some(mut row) = state.filter_rows.row_data(index) else {
            continue;
        };
        let height = if row.paginated {
            FILTER_CAROUSEL_HEIGHT
        } else {
            filter_row_height(&state, &row.chips)
        };
        if (row.height - height).abs() < 0.5 {
            continue;
        }
        row.height = height;
        state.filter_rows.set_row_data(index, row);
    }
    let _ = search;
}

/// Pushes the carousel's page, its page count and where the track sits.
pub(crate) fn push_version(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let search = ui.global::<ContentSearch>();
    search.set_version_page(state.version_page as i32);
    search.set_version_page_count(version_page_count(&state) as i32);
    search.set_version_offset(version_offset(&state));
}

/// Every chip of the version row. The carousel draws the whole track and clips
/// it, so this is the whole list rather than one page.
pub(crate) fn version_chips(state: &ContentController) -> Vec<FilterChip> {
    state
        .version_options
        .iter()
        .map(|option| FilterChip {
            value: SharedString::from(option.as_str()),
            text: SharedString::from(option.as_str()),
            // A version is not in any table; it reads as itself.
            translated: false,
            label_key: SharedString::default(),
            selected: state.form.versions.iter().any(|chosen| chosen == option),
            accent: SharedString::from("minecraft-version"),
        })
        .collect()
}

/// Pushes the filter rows, the carousel and the page range.
pub(crate) fn push_search(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let search = ui.global::<ContentSearch>();

    search.set_version_chips(ModelRc::from(Rc::new(VecModel::from(version_chips(
        &state,
    )))));

    let mut rows: Vec<FilterRow> = Vec::new();
    // Which row draws the paginated carousel rather than a wrapping run of
    // chips. It moves with the loader row, which only some kinds have.
    if state.kind.has_loaders() {
        let chips: Vec<FilterChip> = ["fabric", "forge", "neoforge", "quilt"]
            .iter()
            .map(|key| FilterChip {
                value: SharedString::from(*key),
                text: SharedString::default(),
                translated: true,
                label_key: SharedString::from(*key),
                selected: state.form.loaders.iter().any(|loader| loader == key),
                accent: SharedString::from(*key),
            })
            .collect();
        rows.push(FilterRow {
            label: SharedString::from("loader"),
            chips: ModelRc::from(Rc::new(VecModel::from(chips))),
            paginated: false,
            // Filled in below, once every row is built.
            height: FILTER_CHIP_HEIGHT,
        });
    }
    let version_row = rows.len();
    rows.push(FilterRow {
        label: SharedString::from("version"),
        // Empty on purpose: the carousel draws `ContentSearch.version-chips`,
        // the whole track, which is not a page's worth and so not what a row
        // carries.
        chips: ModelRc::default(),
        paginated: true,
        height: FILTER_CAROUSEL_HEIGHT,
    });

    let mut categories: Vec<FilterChip> = state
        .categories()
        .into_iter()
        .map(|(value, key)| FilterChip {
            value: SharedString::from(value.as_str()),
            text: SharedString::default(),
            translated: true,
            label_key: SharedString::from(key.as_str()),
            selected: state.form.categories.iter().any(|chosen| chosen == &value),
            accent: SharedString::default(),
        })
        .collect();
    categories.push(FilterChip {
        value: SharedString::from("favorites"),
        text: SharedString::default(),
        translated: true,
        label_key: SharedString::from("favorites"),
        selected: state.form.favorites_only,
        accent: SharedString::from("favorites"),
    });
    rows.push(FilterRow {
        label: SharedString::from("category"),
        chips: ModelRc::from(Rc::new(VecModel::from(categories))),
        paginated: false,
        height: FILTER_CHIP_HEIGHT,
    });

    search.set_version_row(version_row as i32);
    let heights: Vec<f32> = rows
        .iter()
        .map(|row| {
            if row.paginated {
                FILTER_CAROUSEL_HEIGHT
            } else {
                filter_row_height(&state, &row.chips)
            }
        })
        .collect();
    let rows: Vec<FilterRow> = rows
        .into_iter()
        .zip(heights)
        .map(|(row, height)| FilterRow { height, ..row })
        .collect();
    drop(state);
    let filter_model = {
        let state = controller();
        let state = state.borrow();
        if state.filter_rows.row_count() != rows.len() {
            state.filter_rows.set_vec(rows);
        } else {
            for (index, row) in rows.into_iter().enumerate() {
                state.filter_rows.set_row_data(index, row);
            }
        }
        Rc::clone(&state.filter_rows)
    };
    search.set_filters(ModelRc::from(filter_model));
    push_version(ui);
}

pub(crate) fn toggle_chip(ui: &App, value: &str) {
    {
        let state = controller();
        let mut state = state.borrow_mut();
        let is_loader = ["fabric", "forge", "neoforge", "quilt"].contains(&value);
        let is_version = state.version_options.iter().any(|option| option == value);
        if is_loader {
            toggle(&mut state.form.loaders, value);
        } else if is_version {
            toggle(&mut state.form.versions, value);
        } else if value == "favorites" {
            state.form.favorites_only = !state.form.favorites_only;
        } else {
            // Selecting a category clears the favourites filter.
            state.form.favorites_only = false;
            toggle(&mut state.form.categories, value);
        }
    }
    push_search(ui);
    run_search(ui, 1);
}

pub(crate) fn toggle(list: &mut Vec<String>, value: &str) {
    match list.iter().position(|item| item == value) {
        Some(index) => {
            list.remove(index);
        }
        None => list.push(value.to_string()),
    }
}

/// The page bar: the numbered buttons with an ellipsis on either side of a long
/// run, and no ellipsis at all up to 15 pages.
pub(crate) fn pagination_pages(total: usize, current: usize) -> Vec<PageButton> {
    let number = |page: usize| PageButton {
        text: SharedString::from(page.to_string()),
        page: page as i32,
    };
    let ellipsis = || PageButton {
        text: SharedString::from("…"),
        page: -1,
    };
    let mut pages = Vec::new();
    if total <= 15 {
        (1..=total).for_each(|page| pages.push(number(page)));
    } else if current <= 7 {
        (1..=13).for_each(|page| pages.push(number(page)));
        pages.push(ellipsis());
        pages.push(number(total));
    } else if current >= total - 6 {
        pages.push(number(1));
        pages.push(ellipsis());
        (total - 12..=total).for_each(|page| pages.push(number(page)));
    } else {
        pages.push(number(1));
        pages.push(ellipsis());
        (current - 5..=current + 5).for_each(|page| pages.push(number(page)));
        pages.push(ellipsis());
        pages.push(number(total));
    }
    pages
}

/// A card and the targets its callbacks need, which only the search knows.
#[derive(Clone)]
pub(crate) struct BuiltCard {
    card: PendingCard,
    target: CardTarget,
}

pub(crate) fn run_search(ui: &App, page: usize) {
    // The request is identified, and every earlier request for this list is
    // invalidated, before anything else happens.
    let (list, request) = {
        let state = controller();
        let mut state = state.borrow_mut();
        state.form.page = page;
        let list = list_key(state.kind, state.platform.key());
        let request = request_key_of(&state, page);
        // The page the user picked is the page the bar shows *now*, before the
        // answer arrives; only the total comes from the result.
        state.pages.entry(list.clone()).or_insert((page, 0)).0 = page;
        state.lists.entry(list.clone()).or_default().token += 1;
        (list, request)
    };

    // The cache is read *before* the loading flag is raised, so a page that has
    // been seen is drawn without the spinner a fresh request shows.
    let cached = {
        let state = controller();
        let state = state.borrow();
        state
            .lists
            .get(&list)
            .and_then(|entry| entry.cache.get(&request))
            .cloned()
    };
    if let Some((cards, total)) = cached {
        show_results(ui, &list, page, cards, total);
        return;
    }

    // The request is composed from a snapshot: the spawn runs on another
    // thread and may not reach the state.
    let (platform, kind, form, token) = {
        let state = controller();
        let state = state.borrow();
        let token = state.lists.get(&list).map(|entry| entry.token).unwrap_or(0);
        (state.platform, state.kind, state.form.clone(), token)
    };
    ui.global::<ContentState>().set_remote_loading(true);
    push_pages(ui);

    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let result = match platform {
            Platform::Modrinth => search_modrinth(kind, &form).await,
            Platform::CurseForge => search_curseforge(kind, &form).await,
        };
        let _ = weak.upgrade_in_event_loop(move |ui| {
            // An answer for a request the user has paged (or switched) away
            // from is dropped, and dropped *before* it is cached.
            let newest = {
                let state = controller();
                let state = state.borrow();
                state
                    .lists
                    .get(&list)
                    .map(|entry| entry.token == token)
                    .unwrap_or(false)
            };
            if !newest {
                return;
            }
            // A failed request draws nothing and caches nothing: the page it was
            // replacing stays on screen, and the next attempt asks the server
            // again rather than reading back an empty page out of the cache.
            let (cards, total) = match result {
                Ok(result) => result,
                Err(error) => {
                    log::error!("content search failed: {error}");
                    ui.global::<ContentState>().set_remote_loading(false);
                    return;
                }
            };
            // The *unfiltered* page is cached: the favourites chip filters what
            // is drawn, not what was requested.
            {
                let state = controller();
                let mut state = state.borrow_mut();
                let request = request_key_of(&state, page);
                state
                    .lists
                    .entry(list.clone())
                    .or_default()
                    .cache
                    .insert(request, (cards.clone(), total));
            }
            show_results(&ui, &list, page, cards, total);
        });
    });
}

/// What one request is identified by, and it has to cover everything the
/// request carries and nothing it does not. The favourites chip is not in it:
/// it filters what comes back, it does not change what is asked for.
pub(crate) fn request_key_of(state: &ContentController, page: usize) -> String {
    request_key(state.kind, state.platform, page, &state.form)
}

pub(crate) fn request_key(
    kind: RemoteKind,
    platform: Platform,
    page: usize,
    form: &SearchForm,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        kind.key(),
        platform.key(),
        page,
        form.query.trim(),
        form.loaders.join(","),
        form.versions.join(","),
        form.categories.join(","),
    )
}

/// Draws a page of results — from the cache or straight off the wire.
///
/// `list` is the list the request was made *for*: a slow answer for a list the
/// user has since switched away from is cached but not drawn, because each list
/// owns its own results.
pub(crate) fn show_results(ui: &App, list: &str, page: usize, cards: Vec<BuiltCard>, total: usize) {
    let (open, favorites_only) = {
        let state = controller();
        let state = state.borrow();
        (
            list_key(state.kind, state.platform.key()) == list,
            state.form.favorites_only,
        )
    };
    if !open {
        return;
    }
    // The favourites chip narrows what is drawn, against the favourites as they
    // are now; the page itself, and the hit count its page numbers come from,
    // are untouched.
    let cards: Vec<BuiltCard> = if favorites_only {
        let state = controller();
        let state = state.borrow();
        cards
            .into_iter()
            .filter(|built| match &built.target {
                CardTarget::Remote { platform, id } => state.is_favorited(*platform, id),
                _ => true,
            })
            .collect()
    } else {
        cards
    };
    let total_pages = total.div_ceil(PAGE_SIZE).max(1);
    {
        let state = controller();
        let mut state = state.borrow_mut();
        state.targets.clear();
        for built in &cards {
            if let CardTarget::Remote { id, .. } = &built.target {
                state.targets.insert(id.clone(), built.target.clone());
            }
        }
        state.pages.insert(list.to_string(), (page, total_pages));
    }
    ui.global::<ContentState>().set_remote_loading(false);
    let ids: Vec<String> = cards
        .iter()
        .filter_map(|built| match &built.target {
            CardTarget::Remote { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    set_cards(
        ui,
        Grid::Remote,
        cards.into_iter().map(|built| built.card).collect(),
    );
    push_pages(ui);
    // A page whose translations are already cached gets them in the same frame;
    // the rest arrive later and are substituted then.
    apply_translations(ui);
    let platform = {
        let state = controller();
        let state = state.borrow();
        state.platform
    };
    ensure_translations(ui, platform, ids);
}

/// `<platform>:<id>` — one cache for both platforms, keyed so their ids cannot
/// collide.
pub(crate) fn translation_key(platform: Platform, id: &str) -> String {
    format!("{}:{id}", platform.key())
}

/// Whether the launcher is showing Chinese, the only locale a translation is
/// asked for in.
pub(crate) fn chinese_locale(ui: &App) -> bool {
    let language = ui.global::<AppConfig>().get_language();
    crate::config_bridge::resolve_locale(language.as_str()).starts_with("zh")
}

/// Fetches the translated descriptions of the projects now on screen, for the
/// ids that are not cached yet. The translation replaces the API's own English
/// text, on the cards and in the detail panel alike.
pub(crate) fn ensure_translations(ui: &App, platform: Platform, ids: Vec<String>) {
    if !chinese_locale(ui) {
        return;
    }
    let missing: Vec<String> = {
        let state = controller();
        let state = state.borrow();
        let mut seen = HashSet::new();
        ids.into_iter()
            .filter(|id| {
                !id.is_empty()
                    && !state
                        .translations
                        .contains_key(&translation_key(platform, id))
                    && seen.insert(id.clone())
            })
            .collect()
    };
    if missing.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let response = match platform {
            Platform::Modrinth => modrinth::get_project_translations(&missing)
                .await
                .map_err(|error| error.to_string()),
            Platform::CurseForge => {
                let ids: Vec<i64> = missing.iter().filter_map(|id| id.parse().ok()).collect();
                curseforge::get_mod_translations(&ids)
                    .await
                    .map_err(|error| error.to_string())
            }
        };
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                log::warn!("failed to fetch the translated descriptions: {error}");
                return;
            }
        };
        // `[{ project_id, translated }]`, or `[{ modid, translated }]` for
        // CurseForge. A project the mirror has no translation for is simply
        // absent.
        let mut found: Vec<(String, String)> = Vec::new();
        for entry in response.as_array().map(Vec::as_slice).unwrap_or_default() {
            let id = entry
                .get("project_id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    entry
                        .get("modid")
                        .and_then(Value::as_i64)
                        .map(|id| id.to_string())
                });
            let translated = entry.get("translated").and_then(Value::as_str);
            if let (Some(id), Some(translated)) = (id, translated)
                && !translated.is_empty()
            {
                found.push((id, translated.to_string()));
            }
        }
        if found.is_empty() {
            return;
        }
        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for (id, translated) in found {
                    state
                        .translations
                        .insert(translation_key(platform, &id), translated);
                }
            }
            apply_translations(&ui);
        });
    });
}

/// Substitutes the translations that have arrived into what is on screen: the
/// remote grid's rows in place — `set_row_data`, so a card keeps its icon, its
/// hover and any animation in flight — and the open detail panel's description.
pub(crate) fn apply_translations(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let platform = state.platform;
    let model = Rc::clone(&state.remote);
    for index in 0..model.row_count() {
        let Some(mut card) = model.row_data(index) else {
            continue;
        };
        let Some(translated) = state
            .translations
            .get(&translation_key(platform, card.id.as_str()))
        else {
            continue;
        };
        if card.description.as_str() == translated {
            continue;
        }
        card.description = SharedString::from(translated.as_str());
        model.set_row_data(index, card);
    }
    let ui_state = ui.global::<ContentState>();
    let detail_id = ui_state.get_detail_id();
    if let Some(translated) = state
        .translations
        .get(&translation_key(platform, detail_id.as_str()))
    {
        ui_state.set_detail_description(SharedString::from(translated.as_str()));
    }
}

/// The loader slugs the cards tag, and that the CurseForge loader map keys on.
pub(crate) const LOADER_SLUGS: [&str; 4] = ["fabric", "forge", "quilt", "neoforge"];

pub(crate) async fn search_modrinth(
    kind: RemoteKind,
    form: &SearchForm,
) -> Result<(Vec<BuiltCard>, usize), String> {
    let params = modrinth::SearchParameters {
        query: Some(form.query.trim().to_string()).filter(|query| !query.is_empty()),
        facets: Some(
            serde_json::to_string(&modrinth_facets(kind, form))
                .map_err(|error| error.to_string())?,
        ),
        index: None,
        offset: Some(form.page.saturating_sub(1) * PAGE_SIZE),
        limit: Some(PAGE_SIZE),
    };
    let response = modrinth::search_projects(&params)
        .await
        .map_err(|error| error.to_string())?;
    let total = response
        .get("total_hits")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let cards = response
        .get("hits")
        .and_then(Value::as_array)
        .map(|hits| {
            hits.iter()
                .filter_map(|hit| modrinth_card(hit, kind))
                .collect()
        })
        .unwrap_or_default();
    Ok((cards, total))
}

/// The facets are Modrinth's: the project type, then one group per filter,
/// where a group is an OR and the groups are ANDed.
pub(crate) fn modrinth_facets(kind: RemoteKind, form: &SearchForm) -> Vec<Vec<String>> {
    let mut facets = vec![vec![format!("project_type:{}", kind.modrinth_type())]];
    if kind.has_loaders() && !form.loaders.is_empty() {
        facets.push(
            form.loaders
                .iter()
                .map(|loader| format!("categories:{loader}"))
                .collect(),
        );
    }
    if !form.versions.is_empty() {
        facets.push(
            form.versions
                .iter()
                .map(|version| format!("versions:{version}"))
                .collect(),
        );
    }
    if !form.categories.is_empty() {
        facets.push(
            form.categories
                .iter()
                .map(|category| format!("categories:{category}"))
                .collect(),
        );
    }
    facets
}

/// One Modrinth hit as a card, or `None` when it carries no project id.
pub(crate) fn modrinth_card(hit: &Value, kind: RemoteKind) -> Option<BuiltCard> {
    let project_id = hit.get("project_id").and_then(Value::as_str)?;
    if project_id.is_empty() {
        return None;
    }
    Some(BuiltCard {
        card: PendingCard {
            id: project_id.to_string(),
            link: format!(
                "https://modrinth.com/{}/{}",
                hit.get("project_type")
                    .and_then(Value::as_str)
                    .unwrap_or(kind.modrinth_type()),
                crate::json::string(hit, "slug"),
            ),
            title: crate::json::string(hit, "title"),
            subtitle: format!("by {}", crate::json::string(hit, "author")),
            has_subtitle: true,
            description: crate::json::string(hit, "description"),
            tags: modrinth_loader_tags(hit, kind),
            icon: hit
                .get("icon_url")
                .and_then(Value::as_str)
                .and_then(fetch_icon),
            action_kind: "favorite",
            ..Default::default()
        },
        target: CardTarget::Remote {
            platform: Platform::Modrinth,
            id: project_id.to_string(),
        },
    })
}

/// The loader tags a Modrinth hit's `categories` earn; a kind without loaders
/// earns none.
pub(crate) fn modrinth_loader_tags(hit: &Value, kind: RemoteKind) -> Vec<PendingTag> {
    if !kind.has_loaders() {
        return Vec::new();
    }
    let Some(categories) = hit.get("categories").and_then(Value::as_array) else {
        return Vec::new();
    };
    LOADER_SLUGS
        .iter()
        .filter(|loader| {
            categories
                .iter()
                .any(|category| category.as_str() == Some(**loader))
        })
        .map(|loader| tag(&capitalize(loader), &format!("loader-{loader}")))
        .collect()
}

pub(crate) async fn search_curseforge(
    kind: RemoteKind,
    form: &SearchForm,
) -> Result<(Vec<BuiltCard>, usize), String> {
    let params = curseforge_search_params(kind, form);
    let response = curseforge::search_mods(&params)
        .await
        .map_err(|error| error.to_string())?;
    let total = response
        .pointer("/pagination/totalCount")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0) as usize;
    let cards = response
        .get("data")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(curseforge_card).collect())
        .unwrap_or_default();
    Ok((cards, total))
}

/// The CurseForge search body. `apply_query` passes a string through verbatim
/// and `Value::to_string`es everything else, so the list parameters are
/// JSON-encoded here.
pub(crate) fn curseforge_search_params(kind: RemoteKind, form: &SearchForm) -> Value {
    let mut params = json!({
        "gameId": curseforge::MINECRAFT_GAME_ID,
        "classId": kind.curseforge_class(),
        "index": form.page.saturating_sub(1) * PAGE_SIZE,
        "pageSize": PAGE_SIZE,
    });
    let object = params.as_object_mut().expect("just built");
    if !form.query.trim().is_empty() {
        object.insert("searchFilter".into(), json!(form.query.trim()));
    }
    if !form.versions.is_empty() {
        let versions: Vec<&String> = form.versions.iter().take(4).collect();
        object.insert(
            "gameVersions".into(),
            json!(serde_json::to_string(&versions).unwrap_or_default()),
        );
    }
    if kind.has_loaders() && !form.loaders.is_empty() {
        let types: Vec<i64> = form
            .loaders
            .iter()
            .filter_map(|loader| curseforge_loader_type(loader))
            .take(5)
            .collect();
        object.insert(
            "modLoaderTypes".into(),
            json!(serde_json::to_string(&types).unwrap_or_default()),
        );
    }
    if !form.categories.is_empty() {
        let ids: Vec<i64> = form
            .categories
            .iter()
            .filter_map(|id| id.parse().ok())
            .collect();
        object.insert(
            "categoryIds".into(),
            json!(serde_json::to_string(&ids).unwrap_or_default()),
        );
    }
    params
}

/// One CurseForge entry as a card, or `None` when it carries no id.
pub(crate) fn curseforge_card(entry: &Value) -> Option<BuiltCard> {
    let project_id = entry.get("id").and_then(Value::as_i64)?.to_string();
    let mut tags = Vec::new();
    if let Some(version) = entry
        .pointer("/latestFilesIndexes/0/gameVersion")
        .and_then(Value::as_str)
    {
        tags.push(tag(version, "version"));
    }
    let authors = entry
        .get("authors")
        .and_then(Value::as_array)
        .map(|authors| {
            authors
                .iter()
                .filter_map(|author| author.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    Some(BuiltCard {
        card: PendingCard {
            id: project_id.clone(),
            link: entry
                .pointer("/links/websiteUrl")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            title: crate::json::string(entry, "name"),
            subtitle: format!("by {authors}"),
            has_subtitle: true,
            description: crate::json::string(entry, "summary"),
            tags,
            icon: entry
                .pointer("/logo/url")
                .and_then(Value::as_str)
                .and_then(fetch_icon),
            action_kind: "favorite",
            ..Default::default()
        },
        target: CardTarget::Remote {
            platform: Platform::CurseForge,
            id: project_id,
        },
    })
}

/// CurseForge's `ModLoaderType` numbering.
pub(crate) fn curseforge_loader_type(loader: &str) -> Option<i64> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

/// The category tables each list offers. A CurseForge entry is `(id, slug)`:
/// the id is what the API filters by and the slug is what names the label. The
/// favourites chip is not here — every list appends it last.
pub(crate) const CURSEFORGE_MOD_CATEGORIES: [(&str, &str); 16] = [
    ("422", "adventure-rpg"),
    ("434", "armor-weapons-tools"),
    ("406", "world-gen"),
    ("412", "technology"),
    ("419", "magic"),
    ("420", "storage"),
    ("421", "library-api"),
    ("423", "map-information"),
    ("5191", "utility-qol"),
    ("435", "server-utility"),
    ("436", "mc-food"),
    ("6814", "performance"),
    ("6821", "bug-fixes"),
    ("4558", "redstone"),
    ("424", "cosmetic"),
    ("425", "mc-miscellaneous"),
];

pub(crate) const MODRINTH_MOD_CATEGORIES: [&str; 19] = [
    "adventure",
    "cursed",
    "decoration",
    "economy",
    "equipment",
    "food",
    "game-mechanics",
    "library",
    "magic",
    "management",
    "minigame",
    "mobs",
    "optimization",
    "social",
    "storage",
    "technology",
    "transportation",
    "utility",
    "worldgen",
];

pub(crate) const CURSEFORGE_RESOURCEPACK_CATEGORIES: [(&str, &str); 19] = [
    ("4252", "crafted"),
    ("4255", "photo-realistic"),
    ("4259", "semi-realistic"),
    ("4256", "simple"),
    ("4258", "traditional"),
    ("4253", "animated"),
    ("4254", "modern"),
    ("4257", "themed"),
    ("4261", "mod-support"),
    ("4264", "rpg"),
    ("4266", "gameplay"),
    ("4268", "gui"),
    ("4269", "sound"),
    ("4270", "environment"),
    ("4271", "world-gen"),
    ("4273", "blocks"),
    ("4274", "items"),
    ("4275", "mobs"),
    ("4276", "weather"),
];

pub(crate) const MODRINTH_RESOURCEPACK_CATEGORIES: [&str; 20] = [
    "faithful",
    "16x",
    "32x",
    "64x",
    "128x",
    "256x",
    "photo-realistic",
    "semi-realistic",
    "simple",
    "modern",
    "theme-based",
    "classic",
    "dark",
    "medieval",
    "anime",
    "cartoon",
    "pixel-art",
    "vanilla-plus",
    "utility",
    "other",
];

pub(crate) const CURSEFORGE_PACK_CATEGORIES: [(&str, &str); 18] = [
    ("4472", "tech"),
    ("4473", "magic"),
    ("4474", "sci-fi"),
    ("4475", "adventure-and-rpg"),
    ("4476", "exploration"),
    ("4477", "mini-game"),
    ("4478", "quests"),
    ("4479", "hardcore"),
    ("4480", "map-based"),
    ("4481", "small-light"),
    ("4482", "extra-large"),
    ("4483", "combat"),
    ("4484", "multiplayer"),
    ("4487", "ftb"),
    ("4736", "skyblock"),
    ("5128", "vanilla-plus"),
    ("7418", "horror"),
    ("9243", "expert"),
];

pub(crate) const MODRINTH_PACK_CATEGORIES: [&str; 10] = [
    "adventure",
    "challenging",
    "combat",
    "kitchen-sink",
    "lightweight",
    "magic",
    "multiplayer",
    "optimization",
    "quests",
    "technology",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrower_row_takes_more_lines() {
        let chips = [60.0, 60.0, 60.0, 60.0, 60.0];
        // 5 x 60 + 4 x 6 = 324 for one line, so 330 fits and 320 does not.
        assert_eq!(filter_row_height_of(330.0, chips.into_iter()), 20.0);
        assert_eq!(
            filter_row_height_of(320.0, chips.into_iter()),
            FILTER_CHIP_HEIGHT * 2.0 + FILTER_CHIP_GAP
        );
        // Two lines of two chips and a third for the last one.
        assert_eq!(
            filter_row_height_of(130.0, chips.into_iter()),
            FILTER_CHIP_HEIGHT * 3.0 + FILTER_CHIP_GAP * 2.0
        );
    }

    /// What one request is identified by: a page that has been seen is only
    /// re-used when everything the request carries is the same. The favourites
    /// chip is deliberately not part of it — it narrows what is drawn, not what
    /// is asked for.
    #[test]
    fn a_request_is_identified_by_what_it_asks_for() {
        let form = SearchForm {
            query: "sodium".into(),
            loaders: vec!["quilt".into()],
            versions: vec!["1.20.1".into()],
            categories: vec!["optimization".into()],
            favorites_only: false,
            page: 1,
        };
        let key = request_key(RemoteKind::Mods, Platform::Modrinth, 1, &form);

        let mut other_page = form.clone();
        other_page.page = 2;
        assert_ne!(
            key,
            request_key(RemoteKind::Mods, Platform::Modrinth, 2, &form)
        );
        assert_ne!(
            key,
            request_key(RemoteKind::Packs, Platform::Modrinth, 1, &form)
        );
        assert_ne!(
            key,
            request_key(RemoteKind::Mods, Platform::CurseForge, 1, &form)
        );

        let mut favorited = form.clone();
        favorited.favorites_only = true;
        assert_eq!(
            key,
            request_key(RemoteKind::Mods, Platform::Modrinth, 1, &favorited)
        );

        let mut queried = form.clone();
        queried.query = "lithium".into();
        assert_ne!(
            key,
            request_key(RemoteKind::Mods, Platform::Modrinth, 1, &queried)
        );
    }

    /// The page state is filed per *list*, so a local list has none of its own
    /// and must never read a remote one's — a local mods grid must not draw
    /// curseforge's page count. A page the user moved to on one list belongs to
    /// that list alone as well.
    #[test]
    fn a_list_only_ever_reads_its_own_pages() {
        let mut pages = HashMap::new();
        pages.insert(list_key(RemoteKind::Mods, "modrinth"), (1, 247));
        pages.insert(list_key(RemoteKind::Mods, "curseforge"), (3, 158));

        assert_eq!(pages_of(&pages, RemoteKind::Mods, "local"), (1, 0));
        assert_eq!(pages_of(&pages, RemoteKind::Mods, "modrinth"), (1, 247));
        assert_eq!(pages_of(&pages, RemoteKind::Mods, "curseforge"), (3, 158));
        // Another kind's list of the same source is a different list too.
        assert_eq!(
            pages_of(&pages, RemoteKind::ResourcePacks, "curseforge"),
            (1, 0)
        );
    }
}
