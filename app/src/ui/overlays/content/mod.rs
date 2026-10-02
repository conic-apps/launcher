// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The content overlays' Slint adapter: the models the panels draw, the
//! presenters that turn the use case's plain cards into `ContentCard`s, and the
//! callbacks registered on the `ContentState` / `ContentSearch` globals.
//!
//! It owns the install and remove actions and the installed check, the page
//! list and the version carousel, and the loading the panels ask for. The
//! browser's UI-neutral session state — the open list, its query, the search
//! cache, the favourites and the open detail — lives in
//! [`crate::usecases::content`] as `ContentSession`, and this controller reaches
//! it through `Deref`.
//!
//! Two conventions it follows from the rest of the app:
//!
//!   * Everything that touches the network or the disk runs on the tokio
//!     runtime (`crate::support::runtime`) and reports back through
//!     `crate::ui::services::report` — Slint is not thread-safe.
//!   * Nothing here composes a *translatable* string. A string pushed into a
//!     model is fixed at the moment it is pushed and would not follow a
//!     language change, where an `@tr` binding re-evaluates; the labels live in
//!     `ContentText` (`ui/globals/content.slint`) and the views resolve them.
//!     Only data that stays untranslated — author names, version numbers,
//!     loader names — is built here.
//!
//! The code is split by concern: the controller, the presenters and the models
//! live here, the callback wiring is in `wiring`, and the per-list work is in
//! `lists`, `search`, `grid`, `body`, `detail`, `acquire` and `icon`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::{Value, json};
use slint::{
    ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel, Weak,
};

use crate::slint_backend::{
    App, AppConfig, CardTag, ContentCard, ContentSearch, ContentState, DeleteSaveState, Dialogs,
    FilterChip, FilterRow, GalleryShot, GameState, MarkdownImage, MdChunk, MdItem, PageButton,
};
use crate::ui::services::report::deliver;
use crate::usecases::generation::Token;
use content::mods::{ModLoader, ResolvedMod};
use instance::InstanceRuntime;

mod acquire;
mod body;
mod detail;
mod grid;
mod icon;
mod lists;
mod presenter;
mod search;
mod wiring;
/// The world map's tile queue. It lives under `content` because the map is
/// opened from a save's card, mirroring `app/ui/overlays/content/world-map.slint`.
pub(crate) mod world_map;

pub(crate) use acquire::*;
pub(crate) use body::*;
pub(crate) use detail::*;
pub(crate) use grid::*;
pub(crate) use icon::*;
pub(crate) use lists::*;
pub(crate) use presenter::*;
pub(crate) use search::*;
pub(crate) use wiring::*;
// The browser's plain data (cards, the search cache, the open query) is
// UI-neutral and lives in the use case. Re-exported here so the rest of the
// module keeps seeing the names it always did; only the Slint-facing `finish*`
// below stays in this layer.
pub(crate) use crate::usecases::content::*;

/// How many results a remote page holds.
pub(crate) const PAGE_SIZE: usize = 20;
/// How many chips one page of the version carousel *steps* by. It is not how
/// many are visible: the track is drawn whole and clipped, so a wider panel
/// shows more.
pub(crate) const VERSIONS_PER_PAGE: usize = 6;
/// The gap between the carousel's chips, which its offset sums.
pub(crate) const VERSION_GAP: f32 = 6.0;

// Cards are at least 290px wide with a 12px gap and `16px 32px 32px 32px` of
// padding. Slint's `GridLayout` does not wrap and a wrapping `FlexboxLayout`
// measures itself at its own preferred width (see `globals/content.slint`), so
// the column count has to come from outside and every card is placed by hand.
pub(crate) const GRID_MIN_CARD: i32 = 290;
pub(crate) const GRID_GAP: i32 = 12;
pub(crate) const GRID_PAD_X: i32 = 32;
pub(crate) const GRID_PAD_TOP: i32 = 16;
pub(crate) const GRID_PAD_BOTTOM: i32 = 32;
/// A card sits 4px right of its column.
pub(crate) const CARD_SHIFT: i32 = 4;
/// The height of a panel's header bar, which the empty-state placeholder is
/// given the room below.
pub(crate) const TITLE_BAR_HEIGHT: i32 = 52;

/// A card's height.
///
/// Every card of one list has the same shape, so it comes to a single number
/// per list. That number is the info block's padding plus its lines, and the
/// line box of each `<p>` is **its own** font size — `line-height: 1` is set on
/// `*`, and a block's strut comes from its own font, not its parent's:
///
///   16 padding + 14 (name) + 2+11+2 (authors) + 2+10+2 (description)
///   + 16 (the tags' line, whose strut *is* the block's 16px) = 75
///
/// Getting this wrong stretches the icon: `img { width: 72px; height: 100% }`
/// makes the icon as tall as the card — near square at 75px — so an over-tall
/// card shows a visibly squeezed one.
pub(crate) const CARD_HEIGHT: i32 = 75;
/// The same sum for the resource-pack cards, which have no authors line.
pub(crate) const CARD_HEIGHT_NO_SUBTITLE: i32 = 60;
/// The saves' cards: a 14px name, the folder name (10px plus its 4px margins)
/// and the tag line, over the same 16px of padding. A save has no description,
/// so that line is absent — which is what `CARD_HEIGHT` subtracts.
pub(crate) const CARD_HEIGHT_SAVES: i32 = 64;

/// Turns a pending card into the Slint card. Runs on the UI thread, which is
/// the only place a model may be made.
pub(crate) fn finish_card(card: PendingCard) -> ContentCard {
    let tags: Vec<CardTag> = card.tags.into_iter().map(finish_tag).collect();
    ContentCard {
        id: SharedString::from(card.id),
        link: SharedString::from(card.link),
        title: SharedString::from(card.title),
        subtitle: SharedString::from(card.subtitle),
        has_subtitle: card.has_subtitle,
        description: SharedString::from(card.description),
        tags: ModelRc::from(Rc::new(VecModel::from(tags))),
        icon: card
            .icon
            .and_then(resolve_icon)
            .or_else(unknown_icon)
            .unwrap_or_default(),
        action_kind: SharedString::from(card.action_kind),
        shows_play: card.shows_play,
        mod_disabled: card.mod_disabled,
        spawn_x: card.spawn.map_or(0, |(x, _)| x),
        spawn_z: card.spawn.map_or(0, |(_, z)| z),
        ..Default::default()
    }
}

/// The Slint tag behind one [`PendingTag`].
fn finish_tag(tag: PendingTag) -> CardTag {
    let (time_kind, hours, month, day, year) = tag.time.unwrap_or(("", 0, 0, 0, 0));
    CardTag {
        text: SharedString::from(tag.text),
        label: SharedString::from(tag.label),
        kind: SharedString::from(tag.kind),
        time_kind: SharedString::from(time_kind),
        time_hours: hours,
        time_month: month,
        time_day: day,
        time_year: year,
    }
}

thread_local! {
    /// The one state.
    ///
    /// It lives here rather than in a value `setup` owns because an
    /// `upgrade_in_event_loop` closure has to be `Send`, and an `Rc` — which is
    /// what a Slint model is — cannot cross into one. Slint's callbacks and the
    /// event-loop closures both run on the UI thread, so both reach it through
    /// [`controller`]; an async task carries only owned values and a
    /// `Weak<App>`.
    static CONTROLLER: Rc<RefCell<ContentController>> =
        Rc::new(RefCell::new(ContentController::new()));
}

/// The controller, for use on the UI thread.
fn controller() -> Rc<RefCell<ContentController>> {
    CONTROLLER.with(Rc::clone)
}

pub(crate) struct ContentController {
    /// The grids. Kept as `VecModel`s rather than reached back through the
    /// globals, so a relayout can rewrite a row in place instead of replacing
    /// the model — which would destroy and re-create every card element.
    saves: Rc<VecModel<ContentCard>>,
    local_mods: Rc<VecModel<ContentCard>>,
    local_resourcepacks: Rc<VecModel<ContentCard>>,
    remote: Rc<VecModel<ContentCard>>,
    /// Every filter chip's measured width, by its value. The wrapping rows are
    /// laid out from these (`filter_row_height`), because Slint measures a
    /// wrapping `FlexboxLayout` at its own "roughly square" preferred width
    /// rather than at the width the row is given.
    filter_widths: HashMap<String, f32>,
    /// The filter rows, as the model the search panel draws. Held here so a
    /// chip's width report can rewrite one row's height in place — replacing
    /// the model would re-create every chip and start the measurement again.
    filter_rows: Rc<VecModel<FilterRow>>,
    /// Which carousel page is showing. Follows the selection when a panel opens
    /// and the pagers from then on.
    version_page: usize,
    /// Each version chip's measured width, index-aligned with
    /// `version_options`. The chips report it (`chip-measured`); the carousel's
    /// offset is their prefix sum.
    version_widths: Vec<f32>,
    /// The width the open panel's grid has, and the panel's height. Reported by
    /// the grid itself (`ContentState.grid-resized`).
    grid_width: i32,
    panel_height: i32,
    /// Which grid is on screen, so a resize knows what to lay out again.
    open_grid: Option<Grid>,
    /// The browser's UI-neutral session state (the open list, its query, the
    /// search cache, the favourites, the open detail). It lives in the use case
    /// so a second frontend can own the same state and run the same logic; this
    /// controller reaches it through `Deref`, so `self.form` and the like still
    /// read as fields.
    state: ContentSession,
}

impl std::ops::Deref for ContentController {
    type Target = ContentSession;
    fn deref(&self) -> &ContentSession {
        &self.state
    }
}

impl std::ops::DerefMut for ContentController {
    fn deref_mut(&mut self) -> &mut ContentSession {
        &mut self.state
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Grid {
    Saves,
    LocalMods,
    LocalResourcePacks,
    Remote,
}

impl ContentController {
    fn new() -> Self {
        Self {
            saves: Rc::new(VecModel::default()),
            local_mods: Rc::new(VecModel::default()),
            local_resourcepacks: Rc::new(VecModel::default()),
            remote: Rc::new(VecModel::default()),
            filter_widths: HashMap::new(),
            filter_rows: Rc::new(VecModel::default()),
            version_page: 0,
            version_widths: Vec::new(),
            grid_width: 0,
            panel_height: 0,
            open_grid: None,
            state: ContentSession::default(),
        }
    }

    fn sync_instance(&mut self, ui: &App) {
        let id = ui.global::<GameState>().get_current_id().to_string();
        if self.instance_id != id {
            self.instance_id = id;
            self.targets.clear();
            // The caches are keyed by URL and path, and a different instance
            // has its own.
            ICONS.with(|icons| icons.borrow_mut().clear());
            SCREENSHOTS.with(|shots| shots.borrow_mut().clear());
        }
    }

    /// The model of one grid.
    fn model(&self, grid: Grid) -> Rc<VecModel<ContentCard>> {
        match grid {
            Grid::Saves => Rc::clone(&self.saves),
            Grid::LocalMods => Rc::clone(&self.local_mods),
            Grid::LocalResourcePacks => Rc::clone(&self.local_resourcepacks),
            Grid::Remote => Rc::clone(&self.remote),
        }
    }

    /// The carousel opens on the page holding the selected version, so it
    /// starts at the instance's own version rather than at the newest release.
    /// Run when a remote list opens, never while paging — the pagers move the
    /// page without touching the selection.
    fn sync_version_page(&mut self) {
        let selected = self.form.versions.first().cloned();
        self.version_page = selected
            .and_then(|current| {
                self.version_options
                    .iter()
                    .position(|option| *option == current)
            })
            .map(|index| index / VERSIONS_PER_PAGE)
            .unwrap_or(0);
    }

    /// The category table of the platform in use, as `(value, label key)`. The
    /// value is what the API wants — a Modrinth slug, a CurseForge numeric id —
    /// and the label key is what `ContentText.category` resolves.
    fn categories(&self) -> Vec<(String, String)> {
        match (self.kind, self.platform) {
            (RemoteKind::Mods, Platform::Modrinth) => modrinth_rows(&MODRINTH_MOD_CATEGORIES),
            (RemoteKind::Mods, Platform::CurseForge) => curseforge_rows(&CURSEFORGE_MOD_CATEGORIES),
            (RemoteKind::ResourcePacks, Platform::Modrinth) => {
                modrinth_rows(&MODRINTH_RESOURCEPACK_CATEGORIES)
            }
            (RemoteKind::ResourcePacks, Platform::CurseForge) => {
                curseforge_rows(&CURSEFORGE_RESOURCEPACK_CATEGORIES)
            }
            (RemoteKind::Packs, Platform::Modrinth) => modrinth_rows(&MODRINTH_PACK_CATEGORIES),
            (RemoteKind::Packs, Platform::CurseForge) => {
                curseforge_rows(&CURSEFORGE_PACK_CATEGORIES)
            }
        }
    }
}

thread_local! {
    /// Icons and screenshots, by URL or path. Both are rebuilt whenever a panel
    /// reopens or a grid relayouts, so without this the work would be repeated
    /// every time.
    static ICONS: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
    static SCREENSHOTS: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
    /// The world a card shows when it has no icon of its own, decoded once.
    static UNKNOWN_ICON: RefCell<Option<Image>> = const { RefCell::new(None) };
}

/// One Modrinth category: its slug is both the filter value and the label key.
fn modrinth_rows(slugs: &[&str]) -> Vec<(String, String)> {
    slugs
        .iter()
        .map(|slug| (slug.to_string(), slug.to_string()))
        .collect()
}

/// One CurseForge category: the numeric id is the filter value, the slug the
/// label key.
fn curseforge_rows(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(id, slug)| (id.to_string(), slug.to_string()))
        .collect()
}
