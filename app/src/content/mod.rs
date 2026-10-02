// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The content overlays' "script": drives `content`, `modrinth` and
//! `curseforge` from what the panels ask for, and pushes the results into the
//! `ContentState` / `ContentSearch` globals.
//!
//! It owns the install and remove actions and the installed check, the
//! favourites, the page list and the version carousel, and the loading the
//! panels ask for.
//!
//! Two conventions it follows from the rest of the app:
//!
//!   * Everything that touches the network or the disk runs on the tokio
//!     runtime (`crate::runtime`) and reports back through
//!     `upgrade_in_event_loop` — Slint is not thread-safe.
//!   * Nothing here composes a *translatable* string. A string pushed into a
//!     model is fixed at the moment it is pushed and would not follow a
//!     language change, where an `@tr` binding re-evaluates; the labels live in
//!     `ContentText` (`ui/globals/content.slint`) and the views resolve them.
//!     Only data that stays untranslated — author names, version numbers,
//!     loader names — is built here.
//!
//! The code is split by concern: the controller and the types it pushes live
//! here, the callback wiring is in `wiring`, and the per-list work is in
//! `lists`, `search`, `grid`, `body`, `detail`, `acquire` and `icon`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::{Value, json};
use slint::{
    ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel, Weak,
};

use crate::report::{Gate, Token, deliver};
use crate::slint_backend::{
    App, AppConfig, CardTag, ContentCard, ContentSearch, ContentState, DeleteSaveState, Dialogs,
    FilterChip, FilterRow, GalleryShot, GameState, MarkdownImage, MdChunk, MdItem, PageButton,
};
use content::mods::remote::RemoteModPlatform;
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

pub(crate) use acquire::*;
pub(crate) use body::*;
pub(crate) use detail::*;
pub(crate) use grid::*;
pub(crate) use icon::*;
pub(crate) use lists::*;
pub(crate) use presenter::*;
pub(crate) use search::*;
pub(crate) use wiring::*;

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

/// Which remote list is showing: the kind and the platform are state, because
/// only one is ever open.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteKind {
    Mods,
    ResourcePacks,
    Packs,
}

impl RemoteKind {
    fn key(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::Packs => "packs",
        }
    }

    /// CurseForge's `classId`: 6 mods, 12 resource packs, 4471 modpacks.
    fn curseforge_class(self) -> i64 {
        match self {
            Self::Mods => 6,
            Self::ResourcePacks => 12,
            Self::Packs => 4471,
        }
    }

    /// The Modrinth `project_type` facet.
    fn modrinth_type(self) -> &'static str {
        match self {
            Self::Mods => "mod",
            Self::ResourcePacks => "resourcepack",
            Self::Packs => "modpack",
        }
    }

    /// Whether the kind's cards carry loader tags and use the loader filter.
    /// A resource pack has no loader.
    fn has_loaders(self) -> bool {
        matches!(self, Self::Mods | Self::Packs)
    }

    /// Where a download of this kind lands, under the instance root.
    fn folder(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::Packs => "modpacks",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "resourcepacks" => Self::ResourcePacks,
            "packs" => Self::Packs,
            _ => Self::Mods,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Modrinth,
    CurseForge,
}

impl Platform {
    fn key(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curseforge",
        }
    }

    fn api(self) -> RemoteModPlatform {
        match self {
            Self::Modrinth => RemoteModPlatform::Modrinth,
            Self::CurseForge => RemoteModPlatform::CurseForge,
        }
    }
}

/// A card as the background work builds it.
///
/// A Slint model is not `Send`, so a `ContentCard` — whose `tags` are a
/// `ModelRc` — cannot be moved into an `upgrade_in_event_loop` closure. The
/// built cards therefore travel as plain data and become `ContentCard`s inside
/// the closure, on the UI thread. An icon travels as a `PendingImage` — a plain
/// RGBA buffer — because Slint's `Image` itself is not `Send` either.
#[derive(Clone, Default)]
pub(crate) struct PendingCard {
    id: String,
    link: String,
    title: String,
    subtitle: String,
    has_subtitle: bool,
    description: String,
    tags: Vec<PendingTag>,
    icon: Option<PendingImage>,
    /// "" | "favorite" | "file"
    action_kind: &'static str,
    shows_play: bool,
    mod_disabled: bool,
    /// A save's spawn point, which its world map opens centred on. Zero on
    /// every other kind of card, and on a save whose `level.dat` has no
    /// `spawn.pos` — which asks the world for its own instead.
    spawn: Option<(i32, i32)>,
}

impl PendingCard {
    /// The Slint card. Runs on the UI thread, which is the only place a model
    /// may be made.
    fn finish(self) -> ContentCard {
        let tags: Vec<CardTag> = self.tags.into_iter().map(PendingTag::finish).collect();
        ContentCard {
            id: SharedString::from(self.id),
            link: SharedString::from(self.link),
            title: SharedString::from(self.title),
            subtitle: SharedString::from(self.subtitle),
            has_subtitle: self.has_subtitle,
            description: SharedString::from(self.description),
            tags: ModelRc::from(Rc::new(VecModel::from(tags))),
            icon: self
                .icon
                .and_then(resolve_icon)
                .or_else(unknown_icon)
                .unwrap_or_default(),
            action_kind: SharedString::from(self.action_kind),
            shows_play: self.shows_play,
            mod_disabled: self.mod_disabled,
            spawn_x: self.spawn.map_or(0, |(x, _)| x),
            spawn_z: self.spawn.map_or(0, |(_, z)| z),
            ..Default::default()
        }
    }
}

/// An image on its way to the UI thread.
///
/// Slint's `Image` is not `Send`, so a decoded image cannot cross into an
/// `upgrade_in_event_loop` closure. The *decode* — the expensive half, and the
/// one that would stall the window — happens on the background thread, and what
/// crosses is its result: a plain buffer. Building the `Image` from it on the
/// UI thread is a copy.
///
/// `pub(crate)` because the command palette shows the same project icons beside
/// its search results, and shares the `ICONS` cache rather than keeping a second
/// copy of every bitmap.
#[derive(Clone)]
pub(crate) struct PendingImage {
    /// The URL or path it came from, which is also its cache key.
    key: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// One tag, before it becomes a Slint struct.
#[derive(Clone)]
pub(crate) struct PendingTag {
    text: String,
    label: String,
    kind: String,
    /// A relative time's `(kind, hours, month, day, year)`, for the saves'
    /// last-played tag — which is translated on the Slint side.
    time: Option<(&'static str, i32, i32, i32, i32)>,
}

impl PendingTag {
    fn finish(self) -> CardTag {
        let (time_kind, hours, month, day, year) = self.time.unwrap_or(("", 0, 0, 0, 0));
        CardTag {
            text: SharedString::from(self.text),
            label: SharedString::from(self.label),
            kind: SharedString::from(self.kind),
            time_kind: SharedString::from(time_kind),
            time_hours: hours,
            time_month: month,
            time_day: day,
            time_year: year,
        }
    }
}

/// What a card's id stands for, so a callback carrying only the id can find the
/// thing behind it. Rebuilt whenever a list is loaded.
#[derive(Clone)]
pub(crate) enum CardTarget {
    Remote {
        platform: Platform,
        id: String,
    },
    /// A local file — a mod jar or a resource pack — whose folder the card
    /// opens.
    Path(String),
    /// A save folder.
    Save(String),
}

/// The open remote list's query and selections, the same shape for all six
/// lists.
#[derive(Clone, Default)]
pub(crate) struct SearchForm {
    query: String,
    loaders: Vec<String>,
    versions: Vec<String>,
    /// Modrinth category slugs, or CurseForge category ids as strings.
    categories: Vec<String>,
    favorites_only: bool,
    page: usize,
}

/// One remote list's search bookkeeping.
///
/// There is one of each per list, and a list that is destroyed and re-created
/// by a source switch keeps the cache it built.
#[derive(Default)]
pub(crate) struct ListSearch {
    /// The request key → what that exact request returned: the cards and the
    /// total hit count the page count comes from. A page that has been seen is
    /// drawn straight away, without the spinner a fresh request shows.
    cache: HashMap<String, (Vec<BuiltCard>, usize)>,
    /// The newest request's number. An answer that belongs to an older request
    /// is dropped rather than drawn over a newer one — which is what rapid
    /// paging needs, since the answers do not come back in order.
    token: u64,
    /// The instance runtime this list last seeded its filters from, as
    /// `"<loader>|<minecraft>"`. A list seeds once per instance and not again,
    /// so switching away and back gives a list with no filters on it while the
    /// cache above survives.
    initialized_for: Option<String>,
}

/// The detail panel that is open, and what its buttons act on.
#[derive(Clone)]
pub(crate) struct OpenDetail {
    platform: Platform,
    kind: RemoteKind,
    id: String,
    /// The installed mods the project resolved to, for the remove button.
    installed_mods: Vec<ResolvedMod>,
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
    /// The game view's current instance. Re-read whenever a panel opens.
    instance_id: String,
    /// `<platform>:<kind>:<id>` — the favourites key.
    favorites: HashSet<String>,
    targets: HashMap<String, CardTarget>,
    kind: RemoteKind,
    platform: Platform,
    /// "" | "local" | "modrinth" | "curseforge"
    source: String,
    form: SearchForm,
    /// The Minecraft release list, newest first. Empty until it arrives.
    version_options: Vec<String>,
    /// `<kind>:<source>` → that list's `(current page, total pages)`.
    ///
    /// Every list has its own pair, and a source switch destroys and re-creates
    /// one of them, so one list's page numbers are never shown against another's
    /// results.
    pages: HashMap<String, (usize, usize)>,
    /// `<kind>:<source>` → that list's own search cache and request token.
    lists: HashMap<String, ListSearch>,
    /// Every filter chip's measured width, by its value. The wrapping rows are
    /// laid out from these (`filter_row_height`), because Slint measures a
    /// wrapping `FlexboxLayout` at its own "roughly square" preferred width
    /// rather than at the width the row is given.
    filter_widths: HashMap<String, f32>,
    /// The filter rows, as the model the search panel draws. Held here so a
    /// chip's width report can rewrite one row's height in place — replacing
    /// the model would re-create every chip and start the measurement again.
    filter_rows: Rc<VecModel<FilterRow>>,
    /// Translated project descriptions, keyed `<platform>:<id>`. Only filled
    /// for a Chinese locale, and only for the ids that have been on screen.
    translations: HashMap<String, String>,
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
    detail: Option<OpenDetail>,
    /// Bumped on every open, so a slow detail response cannot overwrite a newer
    /// panel.
    detail_seq: u64,
    /// Issues the detail download's token and invalidates it on a newer one.
    detail_download_gate: Gate,
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
            instance_id: String::new(),
            favorites: HashSet::new(),
            targets: HashMap::new(),
            kind: RemoteKind::Mods,
            platform: Platform::Modrinth,
            source: "local".into(),
            form: SearchForm::default(),
            version_options: Vec::new(),
            pages: HashMap::new(),
            lists: HashMap::new(),
            filter_widths: HashMap::new(),
            filter_rows: Rc::new(VecModel::default()),
            translations: HashMap::new(),
            version_page: 0,
            version_widths: Vec::new(),
            grid_width: 0,
            panel_height: 0,
            open_grid: None,
            detail: None,
            detail_seq: 0,
            detail_download_gate: Gate::new(),
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

    fn is_favorited(&self, platform: Platform, id: &str) -> bool {
        self.favorites
            .contains(&favorite_key(platform.key(), self.kind.key(), id))
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
