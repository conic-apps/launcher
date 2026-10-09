// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The news overlay's Slint adapter: the models the panel draws, the geometry,
//! the image loading and the changelog viewer.
//!
//! It mirrors `app/ui/overlays/news/`. The browser's UI-neutral session — the
//! two feeds and the filter over them — lives in [`crate::usecases::news`] as
//! `NewsSession`, and this controller reaches it through `Deref`; only the
//! Slint-facing models, the card geometry (which comes from the panel's width)
//! and the network work live here.
//!
//! Two conventions it follows from the rest of the app:
//!
//!   * Everything that touches the network runs on the tokio runtime and
//!     reports back through `crate::ui::services::report` — Slint is not
//!     thread-safe.
//!   * Nothing here composes a *translatable* string. Text that must follow a
//!     language change is resolved in the `.slint` files; only data that stays
//!     untranslated — a version, a category, a date — is built here.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use futures::{StreamExt, stream};
use slint::{ComponentHandle, Image, Model, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, NewsCard, NewsChip, NewsSearch, NewsState};
use crate::support::runtime;
use crate::ui::overlays::content::{cached_icon, fetch_icon, resolve_icon};
use crate::ui::services::report::report;
use crate::usecases::content::PendingImage;
use crate::usecases::news::{NewsKind, NewsSession};

mod body;
mod layout;
mod wiring;

/// How many card images are fetched at once.
const IMAGE_FETCH_CONCURRENCY: usize = 8;

thread_local! {
    /// The one state.
    ///
    /// It lives behind an `Rc` for the same reason the content controller does:
    /// an `upgrade_in_event_loop` closure has to be `Send`, and an `Rc` — which
    /// is what a Slint model is — cannot cross into one. Slint's callbacks and
    /// the event-loop closures both run on the UI thread, so both reach it
    /// through [`controller`]; an async task carries only owned values and a
    /// `Weak<App>`.
    static CONTROLLER: Rc<RefCell<NewsController>> =
        Rc::new(RefCell::new(NewsController::new()));
}

pub(crate) fn controller() -> Rc<RefCell<NewsController>> {
    CONTROLLER.with(Rc::clone)
}

/// The models the flow draws, the layout inputs, and the changelog in view.
pub(crate) struct NewsController {
    session: NewsSession,
    cards: Rc<VecModel<NewsCard>>,
    type_chips: Rc<VecModel<NewsChip>>,
    year_chips: Rc<VecModel<NewsChip>>,
    month_chips: Rc<VecModel<NewsChip>>,
    /// The panel's width, reported by `NewsListPanel`; the card and filter-row
    /// heights are computed from it. Kept as the float Slint reported so a card
    /// height matches the box the `.slint` grid draws around it.
    list_width: f32,
    /// Each card's top edge and height, index-aligned with `cards`. Kept so a
    /// scroll can work out which cards the viewport touches without rebuilding.
    positions: Vec<f32>,
    heights: Vec<f32>,
    /// Each card's image `(url, id)`, index-aligned with `cards`, for a card
    /// that scrolls into view and needs its image fetched then.
    sources: Vec<(String, String)>,
    /// The cards drawn by the last visibility pass, so a scroll only touches
    /// the ones that entered or left rather than every row.
    last_range: std::ops::Range<usize>,
    /// The scroll state the panel reported: how far the flow has scrolled and
    /// how tall the viewport is.
    scroll_offset: f32,
    viewport_height: f32,
    /// Every chip's measured width, one row at a time. The rows wrap, and Slint
    /// cannot measure a wrapping `FlexboxLayout` at the width it is given, so
    /// the chips report their widths and the height is computed from them.
    type_widths: HashMap<String, f32>,
    year_widths: HashMap<String, f32>,
    month_widths: HashMap<String, f32>,
    /// The changelog whose body is in flight, so a body that lands after the
    /// reader moved on is dropped.
    detail_id: Option<String>,
}

impl std::ops::Deref for NewsController {
    type Target = NewsSession;
    fn deref(&self) -> &NewsSession {
        &self.session
    }
}

impl std::ops::DerefMut for NewsController {
    fn deref_mut(&mut self) -> &mut NewsSession {
        &mut self.session
    }
}

impl NewsController {
    fn new() -> Self {
        Self {
            session: NewsSession::default(),
            cards: Rc::new(VecModel::default()),
            type_chips: Rc::new(VecModel::default()),
            year_chips: Rc::new(VecModel::default()),
            month_chips: Rc::new(VecModel::default()),
            list_width: 0.0,
            positions: Vec::new(),
            heights: Vec::new(),
            sources: Vec::new(),
            last_range: 0..0,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            type_widths: HashMap::new(),
            year_widths: HashMap::new(),
            month_widths: HashMap::new(),
            detail_id: None,
        }
    }

    /// The measured widths of one row. `row` is "type" | "year" | "month".
    fn row_widths(&mut self, row: &str) -> &mut HashMap<String, f32> {
        match row {
            "type" => &mut self.type_widths,
            "year" => &mut self.year_widths,
            _ => &mut self.month_widths,
        }
    }

    fn row_widths_ref(&self, row: &str) -> &HashMap<String, f32> {
        match row {
            "type" => &self.type_widths,
            "year" => &self.year_widths,
            _ => &self.month_widths,
        }
    }
}

/// Registers the models and every callback, and gives the panel its first,
/// empty state.
pub fn setup(ui: &App) {
    bind_models(ui);
    wiring::register_callbacks(ui);
}

/// Hands the models to the globals once.
///
/// The controller keeps the `VecModel` handles so a rebuild rewrites the rows in
/// place; replacing the model instead would destroy and re-create every card
/// element, losing its decoded image and its hover.
fn bind_models(ui: &App) {
    let ctrl = controller();
    let ctrl = ctrl.borrow();
    let state = ui.global::<NewsState>();
    state.set_cards(ModelRc::from(Rc::clone(&ctrl.cards)));

    let search = ui.global::<NewsSearch>();
    search.set_type_chips(ModelRc::from(Rc::clone(&ctrl.type_chips)));
    search.set_year_chips(ModelRc::from(Rc::clone(&ctrl.year_chips)));
    search.set_month_chips(ModelRc::from(Rc::clone(&ctrl.month_chips)));
}

/// Opens or closes the overlay. Opening fetches the feeds the first time.
pub(crate) fn toggle(ui: &App) {
    let state = ui.global::<NewsState>();
    if state.get_visible() {
        close(ui);
        return;
    }
    state.set_visible(true);
    state.set_detail_visible(false);
    // A feed that is still in flight should keep showing its spinner when the
    // panel reopens; one that has landed should not.
    state.set_loading(!controller().borrow().is_loaded());
    load(ui);
}

pub(crate) fn close(ui: &App) {
    let state = ui.global::<NewsState>();
    state.set_visible(false);
    state.set_detail_visible(false);
    body::clear(ui);
}

pub(crate) fn close_detail(ui: &App) {
    ui.global::<NewsState>().set_detail_visible(false);
    body::clear(ui);
}

/// Fetches the two feeds, once.
pub(crate) fn load(ui: &App) {
    if !controller().borrow_mut().begin_load() {
        // Already loading or loaded: draw whatever the session has.
        ui.global::<NewsState>()
            .set_loading(!controller().borrow().is_loaded());
        if controller().borrow().is_loaded() {
            push_chips(ui);
            rebuild(ui);
        }
        return;
    }
    ui.global::<NewsState>().set_loading(true);
    let weak = ui.as_weak();
    runtime::spawn(async move {
        // Both fetches are stringified and handed to `finish_load`, which logs
        // the errors — but nothing said *what was asked for*, and the URL only
        // ever appeared inside the request the crate makes. This records the
        // attempt so a failed feed has a fetch attached to it.
        log::info!("Fetching the Mojang news feed and the Java changelog index");
        let news = news::fetch_news().await.map_err(|error| error.to_string());
        let changelogs = news::fetch_changelogs()
            .await
            .map_err(|error| error.to_string());
        report(&weak, move |ui| {
            controller().borrow_mut().finish_load(news, changelogs);
            ui.global::<NewsState>().set_loading(false);
            if ui.global::<NewsState>().get_visible() {
                push_chips(&ui);
                rebuild(&ui);
            }
        });
    });
}

/// One card before it is placed: the model row, and the image it will need.
struct BuiltCard {
    card: NewsCard,
    /// The card's image URL and id, kept beside the row so a card that scrolls
    /// into view later can fetch its image without the model having to carry
    /// the URL.
    url: String,
    id: String,
}

/// Rebuilds the flow from the session's current filter.
///
/// Everything is stacked here — the cards' heights and their `y`, and the range
/// of them a viewport touches — because the flow culls: an off-screen card is
/// neither drawn nor fetched. The heights come from the panel's width and the
/// image ratios; the range comes from the scroll position the panel reported.
pub(crate) fn rebuild(ui: &App) {
    let mut built = build_cards();
    let heights: Vec<f32> = built.iter().map(|item| item.card.height).collect();
    let positions = layout::positions(&heights, layout::FLOW_GAP, layout::FLOW_PAD_TOP);
    let total = layout::total_height(
        &heights,
        layout::FLOW_GAP,
        layout::FLOW_PAD_TOP,
        layout::FLOW_PAD_BOTTOM,
    );

    let range = {
        let ctrl = controller();
        let ctrl = ctrl.borrow();
        layout::visible_range(
            &positions,
            &heights,
            ctrl.scroll_offset,
            ctrl.viewport_height,
            ctrl.viewport_height.max(MIN_VIEWPORT),
        )
    };

    let mut images = ImageQueue::default();
    let cards: Vec<NewsCard> = built
        .iter_mut()
        .enumerate()
        .map(|(index, item)| {
            item.card.y = positions[index];
            item.card.in_view = range.contains(&index);
            if item.card.in_view {
                let (image, loading) = images.take(&item.url, &item.id);
                item.card.image = image;
                item.card.image_loading = loading;
            }
            item.card.clone()
        })
        .collect();

    {
        let ctrl = controller();
        let mut ctrl = ctrl.borrow_mut();
        ctrl.positions = positions;
        ctrl.heights = heights;
        ctrl.sources = built
            .iter()
            .map(|item| (item.url.clone(), item.id.clone()))
            .collect();
        ctrl.last_range = range.clone();
        ctrl.cards.set_vec(cards);
    }
    let state = ui.global::<NewsState>();
    state.set_list_height(total);
    fetch_images(ui, images.pending);
}

/// The cards the active filter produces, before they are placed.
fn build_cards() -> Vec<BuiltCard> {
    let ctrl = controller();
    let ctrl = ctrl.borrow();
    match ctrl.kind() {
        NewsKind::News => ctrl.filtered_news().map(banner_card).collect(),
        NewsKind::Changelog => ctrl.filtered_changelogs().map(changelog_card).collect(),
    }
}

/// One news entry as the flow's banner card. The banner box is a fixed height,
/// so the image's ratio is not needed to size it — it is carried in the model
/// only because the one card struct serves both feeds.
fn banner_card(item: &news::NewsItem) -> BuiltCard {
    let ratio = image_ratio(item.image_width, item.image_height);
    BuiltCard {
        card: NewsCard {
            height: layout::banner_height(),
            y: 0.0,
            in_view: false,
            kind: SharedString::from("news"),
            id: SharedString::from(item.id.as_str()),
            title: SharedString::from(item.title.as_str()),
            subtitle: SharedString::from(item.category.as_str()),
            tag: SharedString::default(),
            date_year: item.year as i32,
            date_month: item.month as i32,
            date_day: item.day as i32,
            description: SharedString::from(item.text.as_str()),
            image: Image::default(),
            image_ratio: ratio,
            image_loading: true,
            link: SharedString::from(item.read_more.as_str()),
        },
        url: item.image_url.clone(),
        id: item.id.clone(),
    }
}

/// One changelog entry as the flow's row card.
fn changelog_card(entry: &news::ChangelogEntry) -> BuiltCard {
    let ratio = image_ratio(entry.image_width, entry.image_height);
    BuiltCard {
        card: NewsCard {
            height: layout::changelog_height(),
            y: 0.0,
            in_view: false,
            kind: SharedString::from("changelog"),
            id: SharedString::from(entry.id.as_str()),
            title: SharedString::from(entry.title.as_str()),
            subtitle: SharedString::from(entry.version.as_str()),
            tag: SharedString::from(entry.kind.key()),
            date_year: entry.year as i32,
            date_month: entry.month as i32,
            date_day: entry.day as i32,
            description: SharedString::from(entry.short_text.as_str()),
            image: Image::default(),
            image_ratio: ratio,
            image_loading: true,
            link: SharedString::default(),
        },
        url: entry.image_url.clone(),
        id: entry.id.clone(),
    }
}

/// A viewport shorter than this is treated as this tall, so a panel that has
/// not reported its size yet still fills its first screen.
const MIN_VIEWPORT: f32 = 600.0;

/// The width-over-height of an image, with a missing dimension treated as
/// square rather than as a division by zero.
fn image_ratio(width: u32, height: u32) -> f32 {
    width as f32 / height.max(1) as f32
}

/// The images a freshly built flow still has to fetch, folded by URL so a
/// repeated image is fetched once.
#[derive(Default)]
struct ImageQueue {
    seen: HashSet<String>,
    /// `(url, card id)`.
    pending: Vec<(String, String)>,
}

impl ImageQueue {
    /// The image to draw now — the cached bitmap, or an empty one while a fetch
    /// is queued — and whether it is still loading.
    fn take(&mut self, url: &str, id: &str) -> (Image, bool) {
        if let Some(cached) = cached_icon(url) {
            return (cached, false);
        }
        if !url.is_empty() && self.seen.insert(url.to_string()) {
            self.pending.push((url.to_string(), id.to_string()));
        }
        (Image::default(), true)
    }
}

/// Fetches the card images, dropping each into its row as it arrives.
fn fetch_images(ui: &App, pending: Vec<(String, String)>) {
    if pending.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    runtime::spawn(async move {
        let fetches = stream::iter(pending).map(|(url, id)| {
            let weak = weak.clone();
            async move {
                // `fetch_icon` blocks on the fetch and the decode inside the
                // task, the same trade the content grid's icons make.
                let image = fetch_icon(&url);
                report(&weak, move |_ui| set_card_image(&id, image));
            }
        });
        fetches
            .buffer_unordered(IMAGE_FETCH_CONCURRENCY)
            .for_each(|()| async {})
            .await;
    });
}

/// Puts one card image into the row that asked for it, if that row is still
/// the card it was.
fn set_card_image(id: &str, image: Option<PendingImage>) {
    let Some(image) = image.and_then(resolve_icon) else {
        return;
    };
    let ctrl = controller();
    let ctrl = ctrl.borrow();
    let model = Rc::clone(&ctrl.cards);
    for index in 0..model.row_count() {
        let Some(mut card) = model.row_data(index) else {
            continue;
        };
        if card.id.as_str() != id {
            continue;
        }
        card.image = image;
        card.image_loading = false;
        model.set_row_data(index, card);
        break;
    }
}

/// Opens what a card stands for: the article in the browser, or the changelog
/// in the panel.
pub(crate) fn open_card(ui: &App, id: &str) {
    enum Open {
        Article(String),
        Changelog(news::ChangelogEntry),
    }
    let open = {
        let ctrl = controller();
        let ctrl = ctrl.borrow();
        match ctrl.kind() {
            NewsKind::News => ctrl
                .news_item(id)
                .map(|item| Open::Article(item.read_more.clone())),
            NewsKind::Changelog => ctrl.changelog(id).cloned().map(Open::Changelog),
        }
    };
    match open {
        Some(Open::Article(url)) => open_url(&url),
        Some(Open::Changelog(entry)) => open_detail(ui, &entry),
        None => {}
    }
}

/// Opens a changelog in the detail panel and fetches its body.
fn open_detail(ui: &App, entry: &news::ChangelogEntry) {
    let state = ui.global::<NewsState>();
    state.set_detail_visible(true);
    state.set_detail_loading(true);
    state.set_detail_title(SharedString::from(entry.title.as_str()));
    state.set_detail_subtitle(SharedString::from(entry.kind.key()));
    state.set_detail_year(entry.year as i32);
    state.set_detail_month(entry.month as i32);
    state.set_detail_day(entry.day as i32);
    state.set_detail_image(Image::default());
    {
        let ctrl = controller();
        ctrl.borrow_mut().detail_id = Some(entry.id.clone());
    }
    body::clear(ui);
    fetch_detail_image(ui, &entry.image_url);

    let weak = ui.as_weak();
    let path = entry.content_path.clone();
    let id = entry.id.clone();
    runtime::spawn(async move {
        let body = news::fetch_changelog_body(&path)
            .await
            .map_err(|error| error.to_string());
        report(&weak, move |ui| {
            // A body that lands after the reader moved to another changelog is
            // dropped.
            if controller().borrow().detail_id.as_deref() != Some(id.as_str()) {
                return;
            }
            match body {
                Ok(html) => body::set_body(&ui, &html),
                Err(error) => log::error!("failed to fetch a changelog body: {error}"),
            }
            ui.global::<NewsState>().set_detail_loading(false);
        });
    });
}

/// Fetches the changelog's square image for the detail header.
fn fetch_detail_image(ui: &App, url: &str) {
    if let Some(cached) = cached_icon(url) {
        ui.global::<NewsState>().set_detail_image(cached);
        return;
    }
    if url.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    let url = url.to_string();
    runtime::spawn(async move {
        let image = fetch_icon(&url);
        report(&weak, move |ui| {
            if let Some(image) = image.and_then(resolve_icon) {
                ui.global::<NewsState>().set_detail_image(image);
            }
        });
    });
}

pub(crate) fn open_url(url: &str) {
    if let Err(error) = crate::ui::services::app_config::open_external(url) {
        log::warn!("failed to open {url}: {error}");
    }
}

/// Rebuilds the three filter rows and their selected state.
pub(crate) fn push_chips(ui: &App) {
    let ctrl = controller();
    let ctrl = ctrl.borrow();

    let selected_kind = ctrl.kind();
    let type_chips: Vec<NewsChip> = [NewsKind::News, NewsKind::Changelog]
        .into_iter()
        .map(|kind| NewsChip {
            value: SharedString::from(kind.key()),
            text: SharedString::default(),
            selected: kind == selected_kind,
        })
        .collect();

    let selected_year = ctrl.year().map(|year| year.to_string());
    let mut year_chips = vec![all_chip(selected_year.is_none())];
    year_chips.extend(ctrl.years().into_iter().map(|year| {
        let value = year.to_string();
        NewsChip {
            selected: selected_year.as_deref() == Some(value.as_str()),
            value: SharedString::from(value.clone()),
            text: SharedString::from(value),
        }
    }));

    let selected_month = ctrl.month().map(|month| month.to_string());
    let mut month_chips = vec![all_chip(selected_month.is_none())];
    month_chips.extend((1..=12).map(|month| {
        let value = month.to_string();
        NewsChip {
            selected: selected_month.as_deref() == Some(value.as_str()),
            value: SharedString::from(value.clone()),
            text: SharedString::from(value),
        }
    }));

    ctrl.type_chips.set_vec(type_chips);
    ctrl.year_chips.set_vec(year_chips);
    ctrl.month_chips.set_vec(month_chips);
    drop(ctrl);
    push_row_heights(ui);
}

fn all_chip(selected: bool) -> NewsChip {
    NewsChip {
        value: SharedString::default(),
        text: SharedString::default(),
        selected,
    }
}

/// A row's height, from the widths the chips reported.
pub(crate) fn row_height(ctrl: &NewsController, row: &str, chips: &VecModel<NewsChip>) -> f32 {
    let widths = ctrl.row_widths_ref(row);
    layout::filter_row_height(
        layout::filter_available(ctrl.list_width),
        (0..chips.row_count())
            .filter_map(|index| chips.row_data(index))
            .map(|chip| widths.get(chip.value.as_str()).copied().unwrap_or(0.0)),
    )
}

/// Writes the three row heights the search panel draws from.
pub(crate) fn push_row_heights(ui: &App) {
    let ctrl = controller();
    let ctrl = ctrl.borrow();
    let search = ui.global::<NewsSearch>();
    search.set_type_row_height(row_height(&ctrl, "type", &ctrl.type_chips));
    search.set_year_row_height(row_height(&ctrl, "year", &ctrl.year_chips));
    search.set_month_row_height(row_height(&ctrl, "month", &ctrl.month_chips));
}

/// The panel reported the size the cards and the filter rows lay out in.
pub(crate) fn list_resized(ui: &App, width: f32, height: f32) {
    {
        let ctrl = controller();
        let mut ctrl = ctrl.borrow_mut();
        ctrl.list_width = width;
        ctrl.viewport_height = height;
    }
    push_row_heights(ui);
    if controller().borrow().is_loaded() {
        rebuild(ui);
    }
}

/// The list scrolled. Only the cards the window touches are drawn and fetched,
/// so a scroll changes which ones those are rather than redrawing the flow.
pub(crate) fn list_scrolled(ui: &App, offset: f32, viewport: f32) {
    let range = {
        let ctrl = controller();
        let mut ctrl = ctrl.borrow_mut();
        ctrl.scroll_offset = offset;
        ctrl.viewport_height = viewport;
        layout::visible_range(
            &ctrl.positions,
            &ctrl.heights,
            offset,
            viewport,
            viewport.max(MIN_VIEWPORT),
        )
    };
    update_visibility(ui, range);
}

/// Draws the cards inside `range` and hides the ones that left it, fetching the
/// images of the cards that just came into view.
///
/// Only the difference from the last pass is touched: a scroll moves the window
/// by a few cards, and rewriting all 415 rows of a changelog on every frame of
/// the glide would be work for nothing.
fn update_visibility(ui: &App, range: std::ops::Range<usize>) {
    let ctrl = controller();
    let mut ctrl = ctrl.borrow_mut();
    let model = Rc::clone(&ctrl.cards);
    let previous = std::mem::replace(&mut ctrl.last_range, range.clone());
    let mut images = ImageQueue::default();

    for index in range.clone() {
        let Some(mut card) = model.row_data(index) else {
            continue;
        };
        if card.in_view {
            continue;
        }
        card.in_view = true;
        if let Some((url, id)) = ctrl.sources.get(index) {
            let (image, loading) = images.take(url, id);
            card.image = image;
            card.image_loading = loading;
        }
        model.set_row_data(index, card);
    }

    for index in previous {
        if range.contains(&index) {
            continue;
        }
        let Some(mut card) = model.row_data(index) else {
            continue;
        };
        if !card.in_view {
            continue;
        }
        card.in_view = false;
        model.set_row_data(index, card);
    }

    drop(ctrl);
    fetch_images(ui, images.pending);
}

/// A chip reported the width it measured, for its row's height.
pub(crate) fn chip_measured(ui: &App, row: &str, value: &str, width: f32) {
    controller()
        .borrow_mut()
        .row_widths(row)
        .insert(value.to_string(), width);
    push_row_heights(ui);
}

/// A filter chip was clicked. The rows are single-select: clicking the selected
/// chip again clears it, and a year or month left over from the other feed is
/// dropped when the kind changes.
pub(crate) fn chip_clicked(ui: &App, row: &str, value: &str) {
    let changed = {
        let ctrl = controller();
        let mut ctrl = ctrl.borrow_mut();
        match row {
            "type" => {
                let kind = NewsKind::from_key(value);
                if ctrl.kind() == kind {
                    false
                } else {
                    ctrl.set_kind(kind);
                    true
                }
            }
            "year" => {
                let next = toggled(ctrl.year(), value.parse().ok());
                ctrl.set_year(next);
                true
            }
            "month" => {
                let next = toggled(ctrl.month(), value.parse().ok());
                ctrl.set_month(next);
                true
            }
            _ => false,
        }
    };
    if changed {
        push_chips(ui);
        rebuild(ui);
    }
}

/// The next selection when a chip is clicked: the value if it differs from the
/// current one, otherwise nothing — so clicking the selected chip clears it.
fn toggled<T: PartialEq>(current: Option<T>, clicked: Option<T>) -> Option<T> {
    if current == clicked { None } else { clicked }
}

/// The search box was committed.
pub(crate) fn search_now(ui: &App) {
    let query = ui.global::<NewsSearch>().get_query().to_string();
    controller().borrow_mut().set_query(query);
    rebuild(ui);
}

/// The changelog body's view reported its width.
pub(crate) fn resize_body(ui: &App, width: f32) {
    body::layout(ui, width);
}
