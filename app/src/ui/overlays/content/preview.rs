// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's four preview rows: the first few icons of each content list.
//!
//! A separate pass from the parse, on purpose. Parsing decides *what* the
//! instance holds and the summary's counts follow from it immediately; the icons
//! are images that have to be fetched and decoded, and making the number wait for
//! the fifth thumbnail would be the wrong trade in a row that is read at a
//! glance. So this is a request, not a load: every icon goes out through the
//! same memo the cards use, so a preview that has been drawn once costs a map
//! lookup and a preview that has not costs a read of an already-parsed list.
//!
//! The rows are kept as `VecModel`s rather than replaced wholesale, so an icon
//! landing fills one circle instead of re-creating all five — which is what
//! would otherwise make the row flicker once per icon.

use futures::{StreamExt, stream};

use super::*;

/// How many icons a preview row shows.
pub(crate) const PREVIEW_ICONS: usize = 5;

/// The model's index for a kind. The order matches `GameState.preview-*`.
fn slot(kind: Kind) -> usize {
    match kind {
        Kind::Saves => 0,
        Kind::Mods => 1,
        Kind::ResourcePacks => 2,
        Kind::Screenshots => 3,
    }
}

thread_local! {
    /// The four rows' models, for the same reason the grids keep theirs: a new
    /// `ModelRc` would destroy and re-create every circle, so an icon landing in
    /// one would make all five flicker. These are on the UI thread only — a
    /// `VecModel` is an `Rc` inside, which is why the grid models live in the
    /// controller and not in a `static`.
    static MODELS: RefCell<[Rc<VecModel<Image>>; 4]> = RefCell::new([
        Rc::new(VecModel::default()),
        Rc::new(VecModel::default()),
        Rc::new(VecModel::default()),
        Rc::new(VecModel::default()),
    ]);
    /// The keys each row is currently drawn from. Two jobs: it is what a landing
    /// icon is checked against so it cannot land in a row that moved on, and it
    /// is what makes a refresh a no-op when the list behind it did not change.
    static DRAWN: RefCell<HashMap<Kind, Vec<String>>> = RefCell::new(HashMap::new());
    /// The saves a row was last asked to read icons for. Their icons are files
    /// in the world folders rather than a field of anything the parse returns,
    /// so this is the only place that pass can be skipped — and it has to be
    /// skipped, because it runs on every game view apply.
    static SAVES: RefCell<Option<(String, Vec<String>)>> = const { RefCell::new(None) };
}

/// The four rows' models. Handed to the summary once and borrowed where needed —
/// cloning the array is four `Rc` bumps, not a copy of anything.
fn models() -> [Rc<VecModel<Image>>; 4] {
    MODELS.with(|models| models.borrow().clone())
}

/// Hands the four models to the summary. Called once, where the rest of the
/// callbacks are registered.
pub(crate) fn push_preview_models(ui: &App) {
    let models = models();
    let state = ui.global::<GameState>();
    state.set_preview_saves(ModelRc::from(Rc::clone(&models[0])));
    state.set_preview_mods(ModelRc::from(Rc::clone(&models[1])));
    state.set_preview_resourcepacks(ModelRc::from(Rc::clone(&models[2])));
    state.set_preview_screenshots(ModelRc::from(Rc::clone(&models[3])));
}

/// Four empty rows, for when no instance is behind them.
///
/// Emptied rather than replaced: the models were handed to `GameState` once, so
/// a `ModelRc::default()` in their place would leave the summary's rows pointing
/// at nothing and this module's writes landing on models nothing draws.
pub(crate) fn clear_rows() {
    for model in models() {
        model.set_vec(Vec::new());
    }
    DRAWN.with(|drawn| drawn.borrow_mut().clear());
    SAVES.with(|saves| *saves.borrow_mut() = None);
}

/// The first five icons of each of the instance's four lists.
///
/// Called whenever a list is parsed and whenever the selection moves. Every
/// source it needs is in the shared cache, so this reads no files of its own
/// except a save's icon, which is a file in the world folder rather than a field
/// of anything the parse returns.
pub(crate) fn refresh_preview_icons(ui: &App, instance: &str) {
    if ui.global::<GameState>().get_current_id() != instance {
        return;
    }
    // Mods, resource packs and screenshots carry the key of their icon in the
    // parsed list already — a `data:` url for the local ones, an `https:` one
    // where an online lookup supplied it, a path for a screenshot.
    row(
        ui,
        instance,
        Kind::Mods,
        cache::mods(instance)
            .map(|mods| {
                mods.iter()
                    .filter_map(|mod_info| mod_info.icon.clone())
                    .take(PREVIEW_ICONS)
                    .collect()
            })
            .unwrap_or_default(),
        Memo::Icons,
    );
    row(
        ui,
        instance,
        Kind::ResourcePacks,
        cache::resourcepacks(instance)
            .map(|packs| {
                packs
                    .iter()
                    .filter_map(|pack| pack.icon.clone())
                    .take(PREVIEW_ICONS)
                    .collect()
            })
            .unwrap_or_default(),
        Memo::Icons,
    );
    row(
        ui,
        instance,
        Kind::Screenshots,
        cache::screenshots(instance)
            .map(|shots| {
                shots
                    .iter()
                    .take(PREVIEW_ICONS)
                    .map(|path| path.to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default(),
        Memo::Screenshots,
    );
    saves(ui, instance);
}

/// A save's icon is the only source here that has to be read before it has a
/// key, so it is its own pass: the folders come from the cache, the `icon.png`
/// read happens on the blocking pool, and from there it is an ordinary url.
fn saves(ui: &App, instance: &str) {
    let folders = cache::saves(instance)
        .map(|levels| {
            levels
                .iter()
                .take(PREVIEW_ICONS)
                .map(|(folder, _)| folder.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // The same five worlds as last time is nothing to redo. Without this the
    // row would re-read every `icon.png` on every search keystroke, because
    // `apply` calls in here whether or not anything about the instance moved.
    let drawn = SAVES.with(|saves| {
        let saves = saves.borrow();
        match saves.as_ref() {
            Some((owner, known)) => owner == instance && *known == folders,
            None => false,
        }
    });
    if drawn {
        return;
    }
    SAVES.with(|saves| *saves.borrow_mut() = Some((instance.to_string(), folders.clone())));
    let weak = ui.as_weak();
    let instance = instance.to_string();
    crate::support::runtime::spawn(async move {
        let mut urls = Vec::new();
        for folder in folders {
            // Blocking: it reads the world's `icon.png` and encodes it.
            let Ok(url) = crate::support::runtime::block_on(content::saves::get_save_icon(
                &instance, &folder,
            )) else {
                continue;
            };
            urls.push(url);
        }
        crate::ui::services::report::report(&weak, move |ui| {
            row(&ui, &instance, Kind::Saves, urls, Memo::Icons);
        });
    });
}

/// Draws one row from its keys, asking for whatever the memo cannot answer.
///
/// The selection is re-checked here, not just at the entry: the saves' pass is
/// async, so its row is drawn a read later and by then the selection may have
/// moved. Redrawing the row with the previous instance's worlds would leave it
/// showing the wrong saves until the next instance came back.
fn row(ui: &App, instance: &str, kind: Kind, keys: Vec<String>, memo: Memo) {
    if ui.global::<GameState>().get_current_id() != instance {
        return;
    }
    // The same keys as last time is the same row, already drawn. Skipping it is
    // what keeps `apply` — which runs on a sort, a group toggle and every
    // keystroke — from tearing down and re-creating five circles each time.
    let drawn = DRAWN.with(|drawn| {
        let drawn = drawn.borrow();
        match drawn.get(&kind) {
            Some(current) => *current == keys,
            None => false,
        }
    });
    if drawn {
        return;
    }
    let model = Rc::clone(&models()[slot(kind)]);
    let mut icons: Vec<Image> = Vec::with_capacity(keys.len());
    let mut missing: Vec<(usize, String)> = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        match memo.get(key) {
            Some(image) => icons.push(image),
            None => {
                icons.push(Image::default());
                missing.push((index, key.clone()));
            }
        }
    }
    DRAWN.with(|drawn| {
        drawn.borrow_mut().insert(kind, keys);
    });
    model.set_vec(icons);
    fetch(ui, instance, kind, missing, memo);
}

/// Asks for the icons the memo could not answer, and drops each one in as it
/// lands.
///
/// Reported one at a time rather than in a batch at the end: the last of five
/// would otherwise hold back the four that are already there.
fn fetch(ui: &App, instance: &str, kind: Kind, missing: Vec<(usize, String)>, memo: Memo) {
    if missing.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    let instance = instance.to_string();
    crate::support::runtime::spawn(async move {
        let fetches = stream::iter(missing).map(|(index, key)| {
            let weak = weak.clone();
            let instance = instance.clone();
            async move {
                let image = memo.fetch(&key);
                (index, weak, instance, key, image)
            }
        });
        fetches
            .buffer_unordered(ICON_FETCH_CONCURRENCY)
            .for_each(|(index, weak, instance, key, image)| async move {
                crate::ui::services::report::report(&weak, move |ui| {
                    arrive(&ui, &instance, kind, index, &key, image, memo);
                });
            })
            .await;
    });
}

/// Fills one circle, if that circle still wants that icon.
///
/// Runs on the UI thread, so the model may be written here. Every guard is the
/// same question — is what came back still on screen? — asked of the three
/// things that can have moved: the selection, the row's own key list, and the
/// number of circles in the model.
fn arrive(
    ui: &App,
    instance: &str,
    kind: Kind,
    index: usize,
    key: &str,
    image: Option<PendingImage>,
    memo: Memo,
) {
    if ui.global::<GameState>().get_current_id() != instance {
        return;
    }
    let drawn = DRAWN.with(|drawn| drawn.borrow().get(&kind).cloned());
    let Some(keys) = drawn else { return };
    if keys.get(index).map(String::as_str) != Some(key) {
        return;
    }
    let model = &models()[slot(kind)];
    if model.row_count() != keys.len() {
        return;
    }
    if let Some(image) = image.and_then(|image| memo.put(image)) {
        model.set_row_data(index, image);
    }
}
