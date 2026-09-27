// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The content overlays' "script": drives `slint-content`, `slint-modrinth` and
//! `slint-curseforge` from what the panels ask for, and pushes the results into
//! the `ContentState` / `ContentSearch` globals.
//!
//! This is the Slint half of `src/overlays/content/`: `useContentActions.ts`
//! (the install and remove actions and the installed check), `useFavorites.ts`,
//! `useSearchPagination.ts` (the page list and the version carousel) and the
//! loading that the Vue spreads over nine components' `onMounted`s.
//!
//! Two conventions it follows from the rest of the port:
//!
//!   * Everything that touches the network or the disk runs on the tokio
//!     runtime (`crate::runtime`) and reports back through
//!     `upgrade_in_event_loop` — Slint is not thread-safe.
//!   * Nothing here composes a *translatable* string. A string pushed into a
//!     model is fixed at the moment it is pushed and would not follow a
//!     language change, where an `@tr` binding re-evaluates; the labels live in
//!     `ContentText` (`ui/globals/content.slint`) and the views resolve them.
//!     Only data the Vue also keeps untranslated — author names, version
//!     numbers, loader names — is built here.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::{Value, json};
use slint::{ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel};

use crate::slint_backend::{
    App, AppConfig, CardTag, ContentCard, ContentSearch, ContentState, FilterChip, FilterRow,
    GalleryShot, GameState, PageButton,
};
use slint_content::mods::remote::RemoteModPlatform;
use slint_content::mods::{ModLoader, ResolvedMod};

/// How many results a remote page holds (`useSearchPagination.ts`'s `PAGE_SIZE`).
const PAGE_SIZE: usize = 20;
/// How many chips one page of the version carousel *steps* by
/// (`useSearchPagination.ts`'s `VERSIONS_PER_PAGE`). It is not how many are
/// visible: the track is drawn whole and clipped, so a wider panel shows more.
const VERSIONS_PER_PAGE: usize = 6;
/// `.filter-chips-track-inner { gap: 6px }`, which `offsetLeft` counts.
const VERSION_GAP: f32 = 6.0;

// ----- the grid's geometry -----
// The Vue's grid is `repeat(auto-fill, minmax(290px, 1fr))` with a 12px gap and
// `16px 32px 32px 32px` of padding, and Slint has no wrapping grid — its only
// multi-column construct is `Row`, which never wraps — so the column count has
// to come from outside in any case and every card is placed by hand.
const GRID_MIN_CARD: i32 = 290;
const GRID_GAP: i32 = 12;
const GRID_PAD_X: i32 = 32;
const GRID_PAD_TOP: i32 = 16;
const GRID_PAD_BOTTOM: i32 = 32;
/// `.content { transform: translateX(4px) }` — a card sits 4px right of its
/// column.
const CARD_SHIFT: i32 = 4;
/// The height of a panel's header bar, which the empty-state placeholder is
/// given the room below.
const TITLE_BAR_HEIGHT: i32 = 52;

/// A card's height.
///
/// The Vue's cards have no height of their own: the grid row stretches them to
/// the tallest, and every card of one list has the same shape, so it comes to a
/// single number per list. That number is the info block's padding plus its
/// lines, and the line box of each `<p>` is **its own** font size — `line-height:
/// 1` is set on `*` (src/assets/styles/main.css), and a block's strut comes from
/// its own font, not its parent's:
///
///   16 padding + 14 (name) + 2+11+2 (authors) + 2+10+2 (description)
///   + 16 (the tags' line, whose strut *is* the block's 16px) = 75
///
/// Getting this wrong stretches the icon: `img { width: 72px; height: 100% }`
/// makes it as tall as the card, so the Vue's is 72x75 (near square) and a card
/// 13px too tall shows a visibly squeezed one.
const CARD_HEIGHT: i32 = 75;
/// The same sum for the resource-pack cards, which have no authors line.
const CARD_HEIGHT_NO_SUBTITLE: i32 = 60;
/// The saves' cards: a 14px name, the folder name (10px plus its 4px margins)
/// and the tag line, over the same 16px of padding. A save has no description,
/// so that line is absent — which is what `CARD_HEIGHT` subtracts.
const CARD_HEIGHT_SAVES: i32 = 64;

/// Which remote list is showing. The Vue has a component per kind per platform;
/// here the kind and the platform are state, because only one is ever open.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RemoteKind {
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
enum Platform {
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
/// the closure, on the UI thread. (`Image` is `Send`, so the decoded icons
/// cross with them.)
#[derive(Clone, Default)]
struct PendingCard {
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
            icon: self.icon.and_then(resolve_icon).unwrap_or_default(),
            action_kind: SharedString::from(self.action_kind),
            shows_play: self.shows_play,
            mod_disabled: self.mod_disabled,
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
#[derive(Clone)]
struct PendingImage {
    /// The URL or path it came from, which is also its cache key.
    key: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// One tag, before it becomes a Slint struct.
#[derive(Clone)]
struct PendingTag {
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
enum CardTarget {
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

/// The open remote list's query and selections — `useSearchPagination.ts`'s
/// refs plus each list's own `selected*` arrays, which are the same shape for
/// all six.
#[derive(Clone, Default)]
struct SearchForm {
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
/// The Vue keeps these as *module-level* variables in each list component —
/// `ContentModsModrinth.vue` declares `modrinthCache` and
/// `modrinthSearchToken`, `ContentModsCurseforge.vue` its own pair — so there
/// is one of each per list, and a list that is destroyed and re-created by a
/// source switch keeps the cache it built.
#[derive(Default)]
struct ListSearch {
    /// `JSON.stringify(params)` → what that exact request returned: the cards
    /// and the total hit count the page count comes from. A page that has been
    /// seen is drawn straight away, without the spinner a fresh request shows.
    cache: HashMap<String, (Vec<BuiltCard>, usize)>,
    /// The newest request's number. `let token = ++searchToken` and the check
    /// after the `await` mean an answer that belongs to an older request is
    /// dropped rather than drawn over a newer one — which is what rapid paging
    /// needs, since the answers do not come back in order.
    token: u64,
    /// The instance runtime this list last seeded its filters from —
    /// `curseForgeInitializedFor`'s `searchInitKey()`, `"<loader>|<minecraft>"`.
    /// A list seeds once per instance and not again, so switching away and back
    /// gives a list with no filters on it (its selections are per-mount in the
    /// Vue) while the cache above survives.
    initialized_for: Option<String>,
}

/// The detail panel that is open, and what its buttons act on.
#[derive(Clone)]
struct OpenDetail {
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

struct ContentController {
    /// The grids. Kept as `VecModel`s rather than reached back through the
    /// globals, so a relayout can rewrite a row in place instead of replacing
    /// the model — which would destroy and re-create every card element.
    saves: Rc<VecModel<ContentCard>>,
    local_mods: Rc<VecModel<ContentCard>>,
    local_resourcepacks: Rc<VecModel<ContentCard>>,
    remote: Rc<VecModel<ContentCard>>,
    /// The game view's current instance. Re-read whenever a panel opens.
    instance_id: String,
    /// `<platform>:<kind>:<id>` — the Vue's `useFavorites` key.
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
    /// The Vue gives every list its own pair of refs — `ContentModsModrinth.vue`
    /// and `ContentModsCurseforge.vue` each declare `currentPage` and
    /// `totalPages`, and a source switch destroys and re-creates one of them —
    /// so one list's page numbers are never shown against another's results.
    pages: HashMap<String, (usize, usize)>,
    /// `<kind>:<source>` → that list's own search cache and request token, the
    /// way each Vue list component has its own.
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
    /// Translated project descriptions, keyed `<platform>:<id>`
    /// (`useDescriptionTranslation`). Only filled for a Chinese locale, and only
    /// for the ids that have been on screen.
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Grid {
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

    /// `syncVersionPageToSelection`: the carousel opens on the page holding the
    /// selected version, so it starts at the instance's own version rather than
    /// at the newest release. Run when a remote list opens, never while paging —
    /// the pagers move the page without touching the selection.
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

/// Registers every content-overlay callback on `ContentState` / `ContentSearch`
/// and takes over `GameState.open-content`.
pub fn setup(ui: &App) {
    // The grids are pushed into the globals once.
    //
    // The controller keeps the `VecModel` handles so a relayout can rewrite a
    // row in place; replacing the model instead would destroy and re-create
    // every card element, losing its hover and its decoded icon.
    {
        let state = controller();
        let state = state.borrow();
        let ui_state = ui.global::<ContentState>();
        ui_state.set_saves(ModelRc::from(Rc::clone(&state.saves)));
        ui_state.set_local_mods(ModelRc::from(Rc::clone(&state.local_mods)));
        ui_state.set_local_resourcepacks(ModelRc::from(Rc::clone(&state.local_resourcepacks)));
        ui_state.set_remote_cards(ModelRc::from(Rc::clone(&state.remote)));
    }

    // `GameState.open-content(kind)` — the instance summary's preview rows.
    // This replaces the placeholder `game.rs` used to install.
    {
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

    // `GameState.open-packs` — the footer's "install pack" button.
    {
        let weak = ui.as_weak();
        ui.global::<GameState>().on_open_packs(move || {
            let Some(ui) = weak.upgrade() else { return };
            {
                let state = controller();
                let mut state = state.borrow_mut();
                state.sync_instance(&ui);
                state.kind = RemoteKind::Packs;
                state.platform = Platform::Modrinth;
                state.source = "modrinth".into();
                state.form = SearchForm::default();
                let list = list_key(state.kind, state.platform.key());
                initialize_list(&mut state, &list);
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
    }

    // The source switcher.
    {
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
                    // The Vue re-mounts the sub-view on every switch, which
                    // resets its query, its page and its selections — the
                    // filters it seeds come back only if this list has not
                    // already been opened on this instance.
                    state.form = SearchForm::default();
                    let list = list_key(state.kind, state.platform.key());
                    initialize_list(&mut state, &list);
                    state.targets.clear();
                }
            }
            ui.global::<ContentState>()
                .set_source(SharedString::from(source.as_str()));
            // The list being shown is the grid a resize re-lays out from here
            // on, and it is re-laid out now — the local list in particular was
            // loaded while another source was open, at whatever width the panel
            // had then.
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
            // A source switch shows the other list's own page state, which for
            // a list that has not searched yet is a hidden bar.
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
    }

    // Closing.
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

    // A remote card opens its detail panel. A local card has no detail view;
    // the Vue's local cards only carry the folder and delete buttons.
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

    // The cards' folder button, and the saves' delete.
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
            // The Vue's `revealItemInDir`: the file manager opens with the file
            // selected, which `open_external` would not do — it launches it.
            if let Err(error) = crate::config_bridge::reveal_in_dir(&path) {
                log::warn!("failed to reveal {path}: {error}");
            }
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
                if let Err(error) = slint_content::saves::delete_save(&instance, &folder).await {
                    log::error!("failed to delete the save {folder}: {error}");
                    return;
                }
                let _ = weak.upgrade_in_event_loop(move |ui| load_saves(&ui));
            });
        });
    }

    // Favourites. Updated optimistically and rolled back if the write fails,
    // which is what `useFavorites.ts` does.
    {
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
                slint_content::favorites::add_favorite(platform.key().into(), kind.into(), id)
            } else {
                slint_content::favorites::remove_favorite(platform.key().into(), kind.into(), id)
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

    // The search form.
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
            // `function goToPage(page) { if (page < 1 || page > totalPages)
            // return ; … }` — with no result yet the count is 0, so every page
            // is out of range and nothing is requested.
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
    // The version chips report the width they measured, which is what the
    // carousel's offset is summed from — the Vue reads `chips[page * 6]
    // .offsetLeft` off the DOM instead. Keyed by the chip's value, because every
    // row's chips send one and an index would land the other rows' widths on the
    // version list's slots.
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

    // The saves' expansion.
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_select_save(move |folder| {
            let Some(ui) = weak.upgrade() else { return };
            select_save(&ui, folder.as_ref());
        });
    }

    // The screenshot viewer.
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

    // The detail panel's buttons.
    {
        let weak = ui.as_weak();
        ui.global::<ContentState>().on_download(move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(detail) = controller().borrow().detail.clone() else {
                return;
            };
            let instance = controller().borrow().instance_id.clone();
            ui.global::<ContentState>().set_detail_operating(true);
            let weak = ui.as_weak();
            crate::runtime::spawn(async move {
                let outcome = install(&instance, &detail).await;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<ContentState>().set_detail_operating(false);
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
            match slint_content::mods::remote::remove_mod_files(&instance, files) {
                Ok(()) => {
                    refresh_installed(&ui);
                    load_local_mods(&ui);
                }
                Err(error) => log::error!("failed to remove content: {error}"),
            }
            ui.global::<ContentState>().set_detail_operating(false);
        });
    }

    // A grid reports the width it has to lay out in.
    {
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
                // The search panel's rows wrap at the panel's width, so a
                // resize changes how many lines each of them takes — and with
                // it the panel's height. In the Vue the DOM re-measures itself;
                // here the row heights were computed for the old width and the
                // rows then sat too close together (or too far apart) until
                // something else re-measured them.
                relayout_filters(&ui);
                relayout(&ui);
            });
    }

    // Favourites are shared by every panel, so they are read once at startup
    // (`useFavorites.ts` keeps the same module-level cache).
    {
        let weak = ui.as_weak();
        crate::runtime::spawn(async move {
            let favorites = slint_content::favorites::list_favorites().unwrap_or_default();
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
            let _ = weak.upgrade_in_event_loop(move |ui| {
                controller().borrow_mut().favorites = keys;
                refresh_favorite_flags(&ui);
            });
        });
    }
}

fn favorite_key(platform: &str, kind: &str, id: &str) -> String {
    format!("{platform}:{kind}:{id}")
}

/// The loader and Minecraft version the remote lists start from
/// (`ensureModrinthInitialized` reads them off the instance's runtime).
/// `ensure…Initialized`: the loader and version the list opens on, seeded from
/// the instance's runtime. `curseForgeInitializedFor` makes it happen once per
/// instance per list — the Vue's `if (…InitializedFor === key) return` — so a
/// list that is switched away from and back is *not* re-seeded: its selections
/// lived in the component, which the `v-if` destroyed.
fn initialize_list(state: &mut ContentController, list: &str) {
    let key = match slint_instance::get_instance_by_id(&state.instance_id) {
        Some(instance) => {
            let runtime = &instance.config.runtime;
            format!(
                "{}|{}",
                runtime
                    .mod_loader_type
                    .as_ref()
                    .map(|loader| loader.to_string())
                    .unwrap_or_default(),
                runtime.minecraft
            )
        }
        None => String::new(),
    };
    let entry = state.lists.entry(list.to_string()).or_default();
    if entry.initialized_for.as_deref() == Some(key.as_str()) {
        return;
    }
    entry.initialized_for = Some(key);
    seed_filters(state);
}

fn seed_filters(state: &mut ContentController) {
    let Some(instance) = slint_instance::get_instance_by_id(&state.instance_id) else {
        return;
    };
    let runtime = &instance.config.runtime;
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

// ---------------------------------------------------------------------------
// the version carousel and the page bar
// ---------------------------------------------------------------------------

/// The Minecraft release list the version filter is built from
/// (`useSearchPagination.ts`'s `loadVersionOptions`), fetched once.
fn ensure_version_options(ui: &App) {
    if !controller().borrow().version_options.is_empty() {
        // Already fetched: the sync still has to run, because every open
        // re-syncs the carousel to the instance's own version.
        controller().borrow_mut().sync_version_page();
        return;
    }
    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let manifest = match slint_install::get_minecraft_version_list().await {
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
        // The Vue sorts by `releaseTime` descending; the ids sort the same way
        // for the release line, and the list carries no 1.x variants that
        // would differ.
        options.sort_by_key(|id| std::cmp::Reverse(version_order(id)));
        options.dedup();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                state.version_options = options;
                // The Vue awaits the manifest before syncing the page, so the
                // selection is only found once this list exists.
                state.sync_version_page();
            }
            push_search(&ui);
        });
    });
}

fn version_order(id: &str) -> (u32, u32, u32) {
    let mut parts = id.split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// The carousel's page count — how many times the track can step.
fn version_page_count(state: &ContentController) -> usize {
    state.version_options.len().div_ceil(VERSIONS_PER_PAGE)
}

/// `chips[page * 6].offsetLeft`: the width of every chip before the page's
/// first, each followed by the track's 6px gap. The widths are the ones the
/// chips reported through `chip-measured`; a chip that has not been laid out
/// yet counts as nothing, and the offset is recomputed when it reports.
fn version_offset(state: &ContentController) -> f32 {
    let first = state.version_page * VERSIONS_PER_PAGE;
    (0..first.min(state.version_widths.len()))
        .map(|index| state.version_widths[index] + VERSION_GAP)
        .sum()
}

/// The key one list's page state is filed under: the Vue keeps a component per
/// kind per source (`ContentModsLocal.vue`, `ContentModsModrinth.vue`,
/// `ContentPacksCurseforge.vue`, …), each with its own `currentPage` and
/// `totalPages`.
///
/// The key is the *source* (`"local" | "modrinth" | "curseforge"`), not the
/// platform: the local lists have no platform of their own, and keying them on
/// whichever one the last remote list happened to set made the local view read
/// that list's page count — the local mods list drew modrinth's 158 pages.
fn list_key(kind: RemoteKind, source: &str) -> String {
    format!("{}:{}", kind.key(), source)
}

/// That list's `(current page, total pages)` — `(1, 0)` for one that has not
/// searched yet, which is what hides its pagination bar.
fn list_pages(state: &ContentController) -> (usize, usize) {
    pages_of(&state.pages, state.kind, &state.source)
}

/// The lookup itself, apart from the controller so the per-list rule can be
/// tested without one.
fn pages_of(
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
fn push_pages(ui: &App) {
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

/// `.filter-chip { height: 20px }` and `.filter-chips { gap: 6px }`.
const FILTER_CHIP_HEIGHT: f32 = 20.0;
const FILTER_CHIP_GAP: f32 = 6.0;
/// `.filter-row`'s carousel: the 26px pager row.
const FILTER_CAROUSEL_HEIGHT: f32 = 26.0;

/// `.filter-chips { flex-wrap: wrap; gap: 6px }` in a row whose label takes
/// 52px and 10px of gap out of the panel's content box — so the chips wrap
/// inside `panel - 48 (padding) - 62 (label and gap)`.
///
/// Returns the row's height: 20px per line with the 6px gap between them.
fn filter_row_height(state: &ContentController, chips: &ModelRc<FilterChip>) -> f32 {
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
fn filter_row_height_of(available: f32, widths: impl Iterator<Item = f32>) -> f32 {
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
fn relayout_filters(ui: &App) {
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
fn push_version(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let search = ui.global::<ContentSearch>();
    search.set_version_page(state.version_page as i32);
    search.set_version_page_count(version_page_count(&state) as i32);
    search.set_version_offset(version_offset(&state));
}

/// Every chip of the version row. The carousel draws the whole track and clips
/// it, the way the Vue does, so this is the whole list rather than one page.
fn version_chips(state: &ContentController) -> Vec<FilterChip> {
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
fn push_search(ui: &App) {
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

fn toggle_chip(ui: &App, value: &str) {
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
            // Selecting a category clears the favourites filter, as the Vue
            // does.
            state.form.favorites_only = false;
            toggle(&mut state.form.categories, value);
        }
    }
    push_search(ui);
    run_search(ui, 1);
}

fn toggle(list: &mut Vec<String>, value: &str) {
    match list.iter().position(|item| item == value) {
        Some(index) => {
            list.remove(index);
        }
        None => list.push(value.to_string()),
    }
}

/// `paginationPages` (`useSearchPagination.ts`): the numbered buttons with an
/// ellipsis on either side of a long run, and no ellipsis at all up to 15 pages.
fn pagination_pages(total: usize, current: usize) -> Vec<PageButton> {
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

// ---------------------------------------------------------------------------
// grids
// ---------------------------------------------------------------------------

/// Lays the cards out and returns the grid's height.
///
/// The cards are positioned by hand because Slint has no wrapping grid; doing it
/// here also means a resize rewrites the model with `set_row_data` and the same
/// elements move, keeping their hover and their decoded icons, where a rebuilt
/// model would re-create every one of them.
fn layout(cards: &mut [ContentCard], grid_width: i32, card_height: i32) -> i32 {
    let content = (grid_width - 2 * GRID_PAD_X).max(0);
    let columns = (((content + GRID_GAP) / (GRID_MIN_CARD + GRID_GAP)).max(1)) as usize;
    let card_width = ((content - (columns as i32 - 1) * GRID_GAP) / columns as i32).max(0);
    for (index, card) in cards.iter_mut().enumerate() {
        let column = (index % columns) as i32;
        let row = (index / columns) as i32;
        card.x = (CARD_SHIFT + column * (card_width + GRID_GAP)) as f32;
        card.y = (row * (card_height + GRID_GAP)) as f32;
        card.width = card_width as f32;
        card.height = card_height as f32;
        card.in_view = true;
    }
    let rows = cards.len().div_ceil(columns).max(1) as i32;
    GRID_PAD_TOP + rows * card_height + (rows - 1) * GRID_GAP + GRID_PAD_BOTTOM
}

/// The model a grid kind draws into.
fn grid_model(ui: &App, grid: Grid) -> ModelRc<ContentCard> {
    let state = ui.global::<ContentState>();
    match grid {
        Grid::Saves => state.get_saves(),
        Grid::LocalMods => state.get_local_mods(),
        Grid::LocalResourcePacks => state.get_local_resourcepacks(),
        Grid::Remote => state.get_remote_cards(),
    }
}

fn grid_card_height(grid: Grid) -> i32 {
    match grid {
        Grid::Saves => CARD_HEIGHT_SAVES,
        Grid::LocalResourcePacks => CARD_HEIGHT_NO_SUBTITLE,
        Grid::LocalMods | Grid::Remote => CARD_HEIGHT,
    }
}

/// Replaces a grid's cards, laying them out for the width the panel has.
///
/// Runs on the UI thread — the cards become Slint structs here, because a model
/// cannot be made anywhere else.
fn set_cards(ui: &App, grid: Grid, pending: Vec<PendingCard>) {
    let (width, panel_height, model) = {
        let state = controller();
        let mut state = state.borrow_mut();
        state.open_grid = Some(grid);
        (state.grid_width, state.panel_height, state.model(grid))
    };
    let ui_state = ui.global::<ContentState>();
    let mut cards: Vec<ContentCard> = pending.into_iter().map(PendingCard::finish).collect();
    let height = if cards.is_empty() {
        // Nothing to place: give the placeholder the room the grid would have
        // taken, so it sits where the cards would have been.
        (panel_height - TITLE_BAR_HEIGHT).max(0)
    } else {
        layout(&mut cards, width, grid_card_height(grid))
    };
    set_grid_height(&ui_state, grid, height);
    if model.row_count() != cards.len() {
        model.set_vec(cards);
    } else {
        for (index, card) in cards.into_iter().enumerate() {
            model.set_row_data(index, card);
        }
    }
}

/// Makes `grid` the one on screen: it becomes the grid a resize re-lays out,
/// and it is re-laid out now, because a grid that was loaded while another list
/// was open still carries the positions it was given at the old panel width.
fn show_grid(ui: &App, grid: Grid) {
    controller().borrow_mut().open_grid = Some(grid);
    relayout(ui);
}

/// Writes one grid's height to that grid's own property: the four grids are
/// laid out independently, and the one on screen must not be given another
/// list's height — see the note on the properties in `globals/content.slint`.
fn set_grid_height(state: &ContentState<'_>, grid: Grid, height: i32) {
    let height = height as f32;
    match grid {
        Grid::Saves => state.set_saves_grid_height(height),
        Grid::LocalMods => state.set_local_mods_grid_height(height),
        Grid::LocalResourcePacks => state.set_local_resourcepacks_grid_height(height),
        Grid::Remote => state.set_remote_grid_height(height),
    }
}

/// Re-runs the open grid's layout. Called when a panel reports a new width — a
/// window resize, or a panel opening at a different size.
fn relayout(ui: &App) {
    let (grid, width, height) = {
        let state = controller();
        let state = state.borrow();
        let Some(grid) = state.open_grid else {
            return;
        };
        (grid, state.grid_width, state.panel_height)
    };
    let model = grid_model(ui, grid);
    let mut cards: Vec<ContentCard> = (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .collect();
    let grid_height = if cards.is_empty() {
        (height - TITLE_BAR_HEIGHT).max(0)
    } else {
        layout(&mut cards, width, grid_card_height(grid))
    };
    set_grid_height(&ui.global::<ContentState>(), grid, grid_height);
    if !cards.is_empty() {
        for (index, card) in cards.into_iter().enumerate() {
            model.set_row_data(index, card);
        }
    }
}

// ---------------------------------------------------------------------------
// local content
// ---------------------------------------------------------------------------

fn load_saves(ui: &App) {
    ui.global::<ContentState>().set_saves_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::runtime::spawn(async move {
        // Gzip and NBT parsing is blocking, so it runs on the blocking pool the
        // way `game.rs` scans the instance folders.
        let levels = crate::runtime::spawn_blocking({
            let instance = instance.clone();
            move || {
                slint_content::saves::get_all_levels(&instance).map(|levels| {
                    levels
                        .into_iter()
                        .map(|(folder, root)| {
                            (folder, slint_content::saves::summarize_level(&root))
                        })
                        .collect::<Vec<_>>()
                })
            }
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();

        let mut cards = Vec::new();
        for (folder, level) in levels {
            let icon = slint_content::saves::get_save_icon(&instance, &folder)
                .await
                .ok()
                .and_then(|data| fetch_icon(&data));
            cards.push(save_card(&folder, &level, icon));
        }
        cards.sort_by(|a, b| a.id.cmp(&b.id));

        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for card in &cards {
                    state
                        .targets
                        .insert(card.id.to_string(), CardTarget::Save(card.id.to_string()));
                }
            }
            set_cards(&ui, Grid::Saves, cards);
            ui.global::<ContentState>().set_saves_loading(false);
        });
    });
}

/// One save's card, from the summary `slint-content` reads out of `level.dat`.
fn save_card(
    folder: &str,
    level: &slint_content::saves::LevelSummary,
    icon: Option<PendingImage>,
) -> PendingCard {
    let name = level.name.clone().unwrap_or_else(|| folder.to_string());
    let cheats = level.allow_commands;
    let last_played = level.last_played;

    let mut tags: Vec<PendingTag> = Vec::new();
    if let Some(game_type) = level.game_type {
        let (kind, text) = match game_type {
            0 => ("game-mode-survival", "Survival"),
            1 => ("game-mode-creative", "Creative"),
            2 => ("game-mode-adventure", "Adventure"),
            3 => ("game-mode-spectator", "Spectator"),
            _ => ("", ""),
        };
        if !kind.is_empty() {
            tags.push(tag(text, kind));
        }
    }
    if cheats {
        tags.push(tag("Cheats", "command-enabled"));
    }
    if last_played.is_some() {
        // The label and the relative time are both translated, so neither can
        // be composed here; the tag carries the parts `GameTime.last-played`
        // needs and the card resolves them.
        let relative = crate::game::relative_time(last_played);
        tags.push(PendingTag {
            text: String::new(),
            label: String::new(),
            kind: "last-played".to_string(),
            time: Some((
                relative.kind,
                relative.hours,
                relative.month,
                relative.day,
                relative.year,
            )),
        });
    }

    PendingCard {
        id: folder.to_string(),
        title: name,
        subtitle: folder.to_string(),
        has_subtitle: true,
        tags,
        icon,
        action_kind: "file",
        shows_play: true,
        ..Default::default()
    }
}

/// The plain tags: a fill and a text, with no label of their own.
fn tag(text: &str, kind: &str) -> PendingTag {
    PendingTag {
        text: text.to_string(),
        label: String::new(),
        kind: kind.to_string(),
        time: None,
    }
}

fn load_local_mods(ui: &App) {
    ui.global::<ContentState>().set_local_mods_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::runtime::spawn(async move {
        let mods = slint_content::mods::remote::parse_mods(&instance).await;
        let cards: Vec<PendingCard> = mods
            .iter()
            // The Vue filters the embedded (jar-in-jar) mods out of the list.
            .filter(|mod_info| !mod_info.embedded)
            .map(local_mod_card)
            .collect();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for card in &cards {
                    state
                        .targets
                        .insert(card.id.to_string(), CardTarget::Path(card.id.to_string()));
                }
            }
            set_cards(&ui, Grid::LocalMods, cards);
            ui.global::<ContentState>().set_local_mods_loading(false);
        });
    });
}

fn local_mod_card(mod_info: &ResolvedMod) -> PendingCard {
    let mut tags: Vec<PendingTag> = Vec::new();
    if mod_info.loader != ModLoader::Unknown {
        let key = loader_key(mod_info.loader);
        tags.push(tag(&capitalize(key), &format!("loader-{key}")));
    }
    if let Some(version) = &mod_info.version {
        tags.push(tag(version, "version"));
    }
    PendingCard {
        id: mod_info.path.to_string_lossy().to_string(),
        title: mod_info.name.clone(),
        subtitle: format!(
            "by {}",
            mod_info
                .authors
                .iter()
                .map(|author| author.name.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ),
        has_subtitle: true,
        description: mod_info.description.clone().unwrap_or_default(),
        tags,
        // A local icon is a base64 data URL and an online one is a plain URL,
        // which is what `merge_remote` leaves behind; both load the same way.
        icon: mod_info.icon.as_deref().and_then(fetch_icon),
        action_kind: "file",
        mod_disabled: mod_info.disabled,
        ..Default::default()
    }
}

fn loader_key(loader: ModLoader) -> &'static str {
    match loader {
        ModLoader::Fabric => "fabric",
        ModLoader::Forge => "forge",
        ModLoader::Quilt => "quilt",
        ModLoader::NeoForge => "neoforge",
        ModLoader::LiteLoader => "liteloader",
        ModLoader::Unknown => "",
    }
}

/// `fabric` → `Fabric`. The Vue does the same to its `ModLoader` enum's name
/// (`mod.loader.charAt(0).toUpperCase() + mod.loader.slice(1)`) and never
/// translates it, so this is not a `ContentText` label.
fn capitalize(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn load_local_resourcepacks(ui: &App) {
    ui.global::<ContentState>()
        .set_local_resourcepacks_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::runtime::spawn(async move {
        // Reading a pack means opening a zip, so it is blocking work.
        let packs = crate::runtime::spawn_blocking({
            let instance = instance.clone();
            move || slint_content::resourcepack::get_instance_resourcepacks(&instance)
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
        let cards: Vec<PendingCard> = packs.iter().map(resourcepack_card).collect();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                for card in &cards {
                    state
                        .targets
                        .insert(card.id.to_string(), CardTarget::Path(card.id.to_string()));
                }
            }
            set_cards(&ui, Grid::LocalResourcePacks, cards);
            ui.global::<ContentState>()
                .set_local_resourcepacks_loading(false);
        });
    });
}

fn resourcepack_card(pack: &slint_content::resourcepack::Resourcepack) -> PendingCard {
    let mut tags: Vec<PendingTag> = Vec::new();
    // `formatRange` in `ContentResourcepacksLocal.vue`.
    let min = pack
        .metadata
        .pointer("/pack/min_format/0")
        .and_then(Value::as_i64);
    let max = pack
        .metadata
        .pointer("/pack/max_format/0")
        .and_then(Value::as_i64);
    let range = match (min, max) {
        (Some(min), Some(max)) if min == max => format!("{min}"),
        (Some(min), None) => format!("{min}+"),
        (Some(min), Some(max)) => format!("{min}-{max}"),
        (None, _) => String::new(),
    };
    if !range.is_empty() {
        tags.push(tag(&range, "version"));
    }
    PendingCard {
        id: pack.path.to_string_lossy().to_string(),
        title: pack.name.clone(),
        description: pack
            .metadata
            .pointer("/pack/description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tags,
        icon: pack.icon.as_deref().and_then(fetch_icon),
        action_kind: "file",
        ..Default::default()
    }
}

/// How many icons a preview row shows (`InstanceSummary.vue`'s `.slice(0, 5)`).
const PREVIEW_ICONS: usize = 5;

/// The four preview rows' icons: the first five saves, mods, resource packs and
/// screenshots of the instance, each decoded here — the game view's rows are a
/// view of the same content the panels show, and every one of these is a local
/// file or a data URL, so nothing is fetched.
///
/// Called whenever the current instance changes (`game.rs` re-reads its counts
/// then).
pub fn refresh_preview_icons(ui: &App, instance_id: &str) {
    let instance = instance_id.to_string();
    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let (saves, mods, packs, shots) = crate::runtime::spawn_blocking({
            let instance = instance.clone();
            move || {
                let mut saves: Vec<PendingImage> = Vec::new();
                if let Ok(levels) = slint_content::saves::get_all_levels(&instance) {
                    let mut folders: Vec<String> = levels.keys().cloned().collect();
                    folders.sort();
                    for folder in folders.into_iter().take(PREVIEW_ICONS) {
                        // Blocking: it reads the level's `icon.png` and encodes it.
                        let Ok(icon) = crate::runtime::block_on(
                            slint_content::saves::get_save_icon(&instance, &folder),
                        ) else {
                            continue;
                        };
                        if let Some(image) = fetch_icon(&icon) {
                            saves.push(image);
                        }
                    }
                }
                // `parse_mods` is the instance-aware one (`parse_folder` wants
                // the folder itself); it is async, so it is driven to
                // completion here — this whole closure is already off the UI
                // thread.
                let mods: Vec<PendingImage> =
                    crate::runtime::block_on(slint_content::mods::remote::parse_mods(&instance))
                        .iter()
                        .filter(|mod_info| !mod_info.embedded)
                        .filter_map(|mod_info| mod_info.icon.as_deref())
                        .take(PREVIEW_ICONS)
                        .filter_map(fetch_icon)
                        .collect();
                let packs: Vec<PendingImage> =
                    slint_content::resourcepack::get_instance_resourcepacks(&instance)
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|pack| pack.icon.as_deref())
                        .take(PREVIEW_ICONS)
                        .filter_map(fetch_icon)
                        .collect();
                let shots: Vec<PendingImage> =
                    slint_content::screenshots::list_screenshots(&instance)
                        .unwrap_or_default()
                        .iter()
                        .take(PREVIEW_ICONS)
                        .filter_map(|path| {
                            let bytes = std::fs::read(path).ok()?;
                            let (width, height, rgba) = decode_to_rgba(&bytes)?;
                            Some(PendingImage {
                                key: path.clone(),
                                width,
                                height,
                                rgba,
                            })
                        })
                        .collect();
                (saves, mods, packs, shots)
            }
        })
        .await
        .unwrap_or_default();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let state = ui.global::<GameState>();
            let images = |pending: Vec<PendingImage>| {
                ModelRc::from(Rc::new(VecModel::from(
                    pending
                        .into_iter()
                        .filter_map(|image| resolve_image(image, &ICONS))
                        .collect::<Vec<Image>>(),
                )))
            };
            state.set_preview_saves(images(saves));
            state.set_preview_mods(images(mods));
            state.set_preview_resourcepacks(images(packs));
            state.set_preview_screenshots(images(shots));
        });
    });
}

fn load_screenshots(ui: &App) {
    ui.global::<ContentState>().set_screenshots_loading(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::runtime::spawn(async move {
        let paths = slint_content::screenshots::list_screenshots(&instance).unwrap_or_default();
        let images: Vec<PendingImage> = paths
            .iter()
            .filter_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                let (width, height, rgba) = decode_to_rgba(&bytes)?;
                Some(PendingImage {
                    key: path.clone(),
                    width,
                    height,
                    rgba,
                })
            })
            .collect();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let shots: Vec<GalleryShot> = images
                .into_iter()
                .filter_map(|image| resolve_gallery_shot_with(image, &SCREENSHOTS))
                .collect();
            let ui_state = ui.global::<ContentState>();
            ui_state.set_screenshots(ModelRc::from(Rc::new(VecModel::from(shots))));
            ui_state.set_screenshots_loading(false);
            ui_state.set_screenshot_index(0);
            if let Some(first) = ui_state.get_screenshots().row_data(0) {
                ui_state.set_current_screenshot(first.image);
            }
        });
    });
}

fn set_screenshot(ui: &App, index: i32) {
    let state = ui.global::<ContentState>();
    let count = state.get_screenshots().row_count() as i32;
    if count == 0 {
        return;
    }
    let index = index.clamp(0, count - 1);
    state.set_screenshot_index(index);
    if let Some(shot) = state.get_screenshots().row_data(index as usize) {
        state.set_current_screenshot(shot.image);
    }
}

/// The Vue's `selectSave`: one card expands and the rest collapse, and the
/// expansion flips upwards when the card sits within 160px of the window's
/// bottom edge.
fn select_save(ui: &App, folder: &str) {
    let state = ui.global::<ContentState>();
    let model = state.get_saves();
    let mut cards: Vec<ContentCard> = (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .collect();
    let flip_threshold = ui.window().size().height as i32 - 160;
    for card in &mut cards {
        let selected = !folder.is_empty() && card.id == folder;
        card.selected = selected;
        card.expand_up = selected && card.y as i32 + card.height as i32 > flip_threshold;
    }
    for (index, card) in cards.into_iter().enumerate() {
        model.set_row_data(index, card);
    }
}

fn refresh_favorite_flags(ui: &App) {
    let state = controller();
    let state = state.borrow();
    let ui_state = ui.global::<ContentState>();
    let model = ui_state.get_remote_cards();
    let mut cards: Vec<ContentCard> = (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .collect();
    let mut changed = false;
    for card in &mut cards {
        let Some(CardTarget::Remote { platform, id }) = state.targets.get(card.id.as_str()) else {
            continue;
        };
        let favorited = state.is_favorited(*platform, id);
        if card.favorited != favorited {
            card.favorited = favorited;
            changed = true;
        }
    }
    if changed {
        for (index, card) in cards.into_iter().enumerate() {
            model.set_row_data(index, card);
        }
    }
    if let Some(detail) = &state.detail {
        ui_state.set_detail_favorited(state.is_favorited(detail.platform, &detail.id));
    }
}

// ---------------------------------------------------------------------------
// remote lists
// ---------------------------------------------------------------------------

/// A card and the targets its callbacks need, which only the search knows.
#[derive(Clone)]
struct BuiltCard {
    card: PendingCard,
    target: CardTarget,
}

fn run_search(ui: &App, page: usize) {
    // `let token = ++searchToken; const params = buildParams(); const cacheKey =
    // JSON.stringify(params);` — the request is identified, and every earlier
    // request for this list is invalidated, before anything else happens.
    let (list, request) = {
        let state = controller();
        let mut state = state.borrow_mut();
        state.form.page = page;
        let list = list_key(state.kind, state.platform.key());
        let request = request_key_of(&state, page);
        // `currentPage.value = page` — the page the user picked is the page the
        // bar shows *now*, before the answer arrives; only the total comes from
        // the result.
        state.pages.entry(list.clone()).or_insert((page, 0)).0 = page;
        state.lists.entry(list.clone()).or_default().token += 1;
        (list, request)
    };

    // `const cached = cache.get(cacheKey); if (cached) { …; return }` — read
    // *before* the loading flag is raised, so a page that has been seen is
    // drawn without the spinner a fresh request shows.
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
            // `if (token !== …SearchToken) return` — an answer for a request the
            // user has paged (or switched) away from is dropped, and dropped
            // *before* it is cached, exactly as the Vue drops it.
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
            // `catch (error) { console.error(error) }` — a failed request draws
            // nothing and caches nothing: the page it was replacing stays on
            // screen, and the next attempt asks the server again rather than
            // reading back an empty page out of the cache.
            let (cards, total) = match result {
                Ok(result) => result,
                Err(error) => {
                    log::error!("content search failed: {error}");
                    ui.global::<ContentState>().set_remote_loading(false);
                    return;
                }
            };
            // `cache.set(cacheKey, result)` — the *unfiltered* page: the
            // favourites chip filters what is drawn, not what was requested.
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

/// What one request is identified by — the Vue's `JSON.stringify(params)`, and
/// it has to cover everything the request carries and nothing it does not. The
/// favourites chip is not in it: it filters what comes back, it does not change
/// what is asked for.
///
/// The fields are in the order the list's `build…Params` writes them.
fn request_key_of(state: &ContentController, page: usize) -> String {
    request_key(state.kind, state.platform, page, &state.form)
}

fn request_key(kind: RemoteKind, platform: Platform, page: usize, form: &SearchForm) -> String {
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
/// user has since switched away from is cached but not drawn, which is what the
/// Vue gets from each list component owning its own `searchResult` ref.
fn show_results(ui: &App, list: &str, page: usize, cards: Vec<BuiltCard>, total: usize) {
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
    // `filterByFavorites(result)` — the favourites chip narrows what is drawn,
    // against the favourites as they are now; the page itself, and the hit count
    // its page numbers come from, are untouched.
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

/// `<platform>:<id>` — the Vue's two caches (`modrinthCache`, `curseforgeCache`)
/// in one, keyed so the two platforms' ids cannot collide.
fn translation_key(platform: Platform, id: &str) -> String {
    format!("{}:{id}", platform.key())
}

/// Whether the launcher is showing Chinese, which is the only locale the Vue
/// asks for a translation in (`i18n.locale.value.startsWith("zh")`).
fn chinese_locale(ui: &App) -> bool {
    let language = ui.global::<AppConfig>().get_language();
    crate::config_bridge::resolve_locale(language.as_str()).starts_with("zh")
}

/// `useDescriptionTranslation`: fetches the translated descriptions of the
/// projects now on screen, for the ids that are not cached yet. The translation
/// is what the Vue shows in place of the API's own English text, on the cards
/// and in the detail panel alike.
fn ensure_translations(ui: &App, platform: Platform, ids: Vec<String>) {
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
            Platform::Modrinth => slint_modrinth::get_project_translations(&missing)
                .await
                .map_err(|error| error.to_string()),
            Platform::CurseForge => {
                let ids: Vec<i64> = missing.iter().filter_map(|id| id.parse().ok()).collect();
                slint_curseforge::get_mod_translations(&ids)
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
fn apply_translations(ui: &App) {
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

async fn search_modrinth(
    kind: RemoteKind,
    form: &SearchForm,
) -> Result<(Vec<BuiltCard>, usize), String> {
    // The facets are Modrinth's: the project type, then one group per filter,
    // where a group is an OR and the groups are ANDed.
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{}", kind.modrinth_type())]];
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
    let params = slint_modrinth::SearchParameters {
        query: Some(form.query.trim().to_string()).filter(|query| !query.is_empty()),
        facets: Some(serde_json::to_string(&facets).map_err(|error| error.to_string())?),
        index: None,
        offset: Some(form.page.saturating_sub(1) * PAGE_SIZE),
        limit: Some(PAGE_SIZE),
    };
    let response = slint_modrinth::search_projects(&params)
        .await
        .map_err(|error| error.to_string())?;
    let total = response
        .get("total_hits")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;

    let mut cards = Vec::new();
    for hit in response
        .get("hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let project_id = hit
            .get("project_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if project_id.is_empty() {
            continue;
        }
        let mut tags = Vec::new();
        if kind.has_loaders()
            && let Some(categories) = hit.get("categories").and_then(Value::as_array)
        {
            for loader in ["fabric", "forge", "quilt", "neoforge"] {
                if categories
                    .iter()
                    .any(|category| category.as_str() == Some(loader))
                {
                    tags.push(tag(&capitalize(loader), &format!("loader-{loader}")));
                }
            }
        }
        cards.push(BuiltCard {
            card: PendingCard {
                id: project_id.clone(),
                link: format!(
                    "https://modrinth.com/{}/{}",
                    hit.get("project_type")
                        .and_then(Value::as_str)
                        .unwrap_or(kind.modrinth_type()),
                    hit.get("slug").and_then(Value::as_str).unwrap_or_default(),
                ),
                title: hit
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                subtitle: format!(
                    "by {}",
                    hit.get("author")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                has_subtitle: true,
                description: hit
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tags,
                icon: hit
                    .get("icon_url")
                    .and_then(Value::as_str)
                    .and_then(fetch_icon),
                action_kind: "favorite",
                ..Default::default()
            },
            target: CardTarget::Remote {
                platform: Platform::Modrinth,
                id: project_id,
            },
        });
    }
    Ok((cards, total))
}

async fn search_curseforge(
    kind: RemoteKind,
    form: &SearchForm,
) -> Result<(Vec<BuiltCard>, usize), String> {
    let mut params = json!({
        "gameId": slint_curseforge::MINECRAFT_GAME_ID,
        "classId": kind.curseforge_class(),
        "index": form.page.saturating_sub(1) * PAGE_SIZE,
        "pageSize": PAGE_SIZE,
    });
    let object = params.as_object_mut().expect("just built");
    if !form.query.trim().is_empty() {
        object.insert("searchFilter".into(), json!(form.query.trim()));
    }
    // `apply_query` passes a string through verbatim and `Value::to_string`es
    // everything else, so the list parameters are JSON-encoded here — which is
    // what the Vue's `JSON.stringify` does to the same fields.
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

    let response = slint_curseforge::search_mods(&params)
        .await
        .map_err(|error| error.to_string())?;
    let total = response
        .pointer("/pagination/totalCount")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0) as usize;

    let mut cards = Vec::new();
    for mod_info in response
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let Some(id) = mod_info.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let project_id = id.to_string();
        let mut tags = Vec::new();
        if let Some(version) = mod_info
            .pointer("/latestFilesIndexes/0/gameVersion")
            .and_then(Value::as_str)
        {
            tags.push(tag(version, "version"));
        }
        cards.push(BuiltCard {
            card: PendingCard {
                id: project_id.clone(),
                link: mod_info
                    .pointer("/links/websiteUrl")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                title: mod_info
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                subtitle: format!(
                    "by {}",
                    mod_info
                        .get("authors")
                        .and_then(Value::as_array)
                        .map(|authors| authors
                            .iter()
                            .filter_map(|author| author.get("name").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join(","))
                        .unwrap_or_default()
                ),
                has_subtitle: true,
                description: mod_info
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tags,
                icon: mod_info
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
        });
    }
    Ok((cards, total))
}

/// CurseForge's `ModLoaderType` numbering (`index.ts`'s enum).
fn curseforge_loader_type(loader: &str) -> Option<i64> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// detail panels
// ---------------------------------------------------------------------------

fn open_detail(ui: &App, platform: Platform, id: String) {
    let (seq, kind) = {
        let state = controller();
        let mut state = state.borrow_mut();
        state.detail_seq += 1;
        state.detail = Some(OpenDetail {
            platform,
            kind: state.kind,
            id: id.clone(),
            installed_mods: Vec::new(),
        });
        (state.detail_seq, state.kind)
    };
    let ui_state = ui.global::<ContentState>();
    ui_state.set_open_detail(SharedString::from(platform.key()));
    ui_state.set_detail_id(SharedString::from(id.as_str()));
    ui_state.set_detail_loading(true);
    // Only a mod panel has an installed state; a pack or resource-pack one only
    // ever draws the download button.
    ui_state.set_detail_can_remove(kind == RemoteKind::Mods);
    // The Modrinth panels render the body's box unconditionally, the CurseForge
    // ones behind `v-if="modDescription"` — an empty body still draws an empty
    // surface0 box on the first, and nothing on the second.
    ui_state.set_detail_body_always(platform == Platform::Modrinth);
    ui_state.set_detail_installed(false);
    ui_state.set_detail_installed_version(SharedString::default());
    ui_state.set_detail_favorited(false);
    ui_state.set_detail_body(SharedString::default());
    ui_state.set_detail_gallery(ModelRc::default());

    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let loaded = load_detail(platform, &id).await;
        let _ = weak.upgrade_in_event_loop(move |ui| {
            // A different panel has been opened since this request started.
            if controller().borrow().detail_seq != seq {
                return;
            }
            ui.global::<ContentState>().set_detail_loading(false);
            match loaded {
                Ok(loaded) => {
                    let ui_state = ui.global::<ContentState>();
                    ui_state.set_detail_title(SharedString::from(loaded.title));
                    ui_state
                        .set_detail_icon(loaded.icon.and_then(resolve_icon).unwrap_or_default());
                    ui_state.set_detail_source(SharedString::from(loaded.source_url));
                    ui_state.set_detail_source_label(SharedString::from(loaded.source_label));
                    ui_state.set_detail_source_is_github(loaded.source_is_github);
                    ui_state.set_detail_downloads(SharedString::from(loaded.downloads));
                    ui_state.set_detail_followers(SharedString::from(loaded.followers));
                    ui_state.set_detail_has_followers(loaded.has_followers);
                    ui_state.set_detail_description(SharedString::from(loaded.description));
                    ui_state.set_detail_body(SharedString::from(loaded.body));
                    ui_state.set_detail_gallery(ModelRc::from(Rc::new(VecModel::from(
                        loaded
                            .gallery
                            .into_iter()
                            .filter_map(resolve_gallery_shot)
                            .collect::<Vec<GalleryShot>>(),
                    ))));
                }
                Err(error) => log::error!("failed to load the detail panel: {error}"),
            }
            refresh_favorited(&ui);
            refresh_installed(&ui);
            ensure_translations(&ui, platform, vec![id]);
        });
    });
}

struct LoadedDetail {
    title: String,
    icon: Option<PendingImage>,
    source_url: String,
    source_label: String,
    source_is_github: bool,
    downloads: String,
    followers: String,
    has_followers: bool,
    description: String,
    body: String,
    gallery: Vec<PendingImage>,
}

async fn load_detail(platform: Platform, id: &str) -> Result<LoadedDetail, String> {
    match platform {
        Platform::Modrinth => {
            let project = slint_modrinth::get_project(id)
                .await
                .map_err(|error| error.to_string())?;
            let source_url = project
                .get("source_url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            // `formatGithubRepo`: a GitHub URL is shown as `owner/repo`.
            let github = github_repo(&source_url);
            let gallery: Vec<PendingImage> = project
                .get("gallery")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("url").and_then(Value::as_str))
                        .filter_map(fetch_icon)
                        .collect()
                })
                .unwrap_or_default();
            Ok(LoadedDetail {
                title: project
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                icon: project
                    .get("icon_url")
                    .and_then(Value::as_str)
                    .and_then(fetch_icon),
                source_label: github.clone().unwrap_or_else(|| source_url.clone()),
                source_is_github: github.is_some(),
                source_url,
                downloads: number_label(project.get("downloads").and_then(Value::as_i64)),
                followers: number_label(project.get("followers").and_then(Value::as_i64)),
                has_followers: project
                    .get("followers")
                    .is_some_and(|value| !value.is_null()),
                description: project
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                // The README, flattened for the plain-text body renderer (see
                // the README's "Rendering the README bodies").
                body: markdown_to_text(
                    project
                        .get("body")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
                .trim()
                .to_string(),
                gallery,
            })
        }
        Platform::CurseForge => {
            let mod_id: i64 = id
                .parse()
                .map_err(|_| "not a CurseForge mod id".to_string())?;
            let response = slint_curseforge::get_mod(mod_id)
                .await
                .map_err(|error| error.to_string())?;
            let mod_info = response.get("data").cloned().unwrap_or(Value::Null);
            let source_url = mod_info
                .pointer("/links/sourceUrl")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let description =
                slint_curseforge::get_mod_description(mod_id, &json!({ "markup": true }))
                    .await
                    .map_err(|error| error.to_string())?;
            let html = description
                .get("data")
                .and_then(Value::as_str)
                .unwrap_or_default();
            // The Vue refuses a description carrying a `<script>` or `<style>`
            // block outright instead of sanitising it.
            let unsafe_html = {
                let lower = html.to_lowercase();
                lower.contains("<script") || lower.contains("<style")
            };
            let thumbs_up = mod_info.get("thumbsUpCount").and_then(Value::as_i64);
            Ok(LoadedDetail {
                title: mod_info
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                icon: mod_info
                    .pointer("/logo/url")
                    .and_then(Value::as_str)
                    .and_then(fetch_icon),
                source_label: source_url.clone(),
                source_is_github: github_repo(&source_url).is_some(),
                source_url,
                downloads: number_label(mod_info.get("downloadCount").and_then(Value::as_i64)),
                followers: number_label(thumbs_up),
                // The Vue only draws the thumbs-up entry when it is non-zero.
                has_followers: thumbs_up.is_some_and(|count| count > 0),
                description: mod_info
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                body: if unsafe_html {
                    String::new()
                } else {
                    markdown_to_text(html).trim().to_string()
                },
                // The Vue's galleries are a Modrinth-only feature.
                gallery: Vec::new(),
            })
        }
    }
}

/// `formatGithubRepo` (`ContentModrinth*Details.vue`): `owner/repo`, when the
/// URL is a GitHub one. The Vue's own hostname check compares against
/// `"://github.com"` as well, which can never match a parsed host; only the
/// real host is tested here.
fn github_repo(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    if parsed.host_str() != Some("github.com") {
        return None;
    }
    let segments: Vec<&str> = parsed
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    let repo = segments[1].strip_suffix(".git").unwrap_or(segments[1]);
    Some(format!("{}/{}", segments[0], repo))
}

fn number_label(value: Option<i64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// Flattens a Markdown or HTML body to plain text.
///
/// Stands in for the `marked` → `v-html` pipeline the Vue renders with. Tags are
/// dropped, a block-level tag's close becomes a line break, runs of blank lines
/// collapse to one, and the entities the two APIs actually emit are unescaped.
/// The Markdown's own inline syntax is taken off too — emphasis, backticks, and
/// a link or an image's brackets — because the Vue shows the *rendered* result
/// and a body full of `**` and `](https://…)` reads as a document that failed to
/// load. The Slint README's "Rendering the README bodies" sets out what a real
/// renderer would take.
fn markdown_to_text(body: &str) -> String {
    const BLOCK_TAGS: [&str; 17] = [
        "p",
        "div",
        "br",
        "li",
        "ul",
        "ol",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "pre",
        "blockquote",
        "tr",
        "table",
        "hr",
    ];
    let mut out = String::with_capacity(body.len());
    let mut tag = String::new();
    let mut in_tag = false;
    for ch in body.chars() {
        match ch {
            '<' if !in_tag => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                // `</p>` and `<br/>` alike: the leading slash marks the close and
                // the trailing one the self-closing form, and neither is part of
                // the name.
                let name = tag.trim().trim_matches('/').trim().to_lowercase();
                let name = name.split_whitespace().next().unwrap_or("");
                if BLOCK_TAGS.contains(&name) && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ if in_tag => tag.push(ch),
            _ => out.push(ch),
        }
    }
    let text = unescape_entities(&out);
    let text = strip_inline_markdown(&text);
    let mut collapsed = String::with_capacity(text.len());
    let mut blank = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        collapsed.push_str(line.trim_end());
        collapsed.push('\n');
    }
    collapsed
}

/// The entities the two APIs' bodies actually carry, plus the numeric forms.
/// The Vue's webview resolves these; nothing here does, so they are replaced by
/// hand.
fn unescape_entities(text: &str) -> String {
    const NAMED: [(&str, &str); 12] = [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&nbsp;", " "),
        ("&bull;", "•"),
        ("&middot;", "·"),
        ("&mdash;", "—"),
        ("&ndash;", "–"),
        ("&hellip;", "…"),
        ("&times;", "×"),
    ];
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let end = rest.find(';').filter(|end| *end <= 10);
        let Some(end) = end else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[..=end];
        if let Some((_, replacement)) = NAMED.iter().find(|(name, _)| *name == entity) {
            out.push_str(replacement);
        } else if let Some(code) = entity
            .strip_prefix("&#")
            .and_then(|body| body.strip_suffix(';'))
            .and_then(|body| match body.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => body.parse::<u32>().ok(),
            })
            .and_then(char::from_u32)
        {
            out.push(code);
        } else {
            out.push_str(entity);
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Takes the Markdown's inline syntax off, keeping the words: `**bold**` reads
/// as bold, `` `code` `` as code, `[label](url)` as its label, and an image —
/// which this renderer cannot draw — disappears with its alt text.
///
/// Stands in for `marked`'s parse. It is deliberately forgiving rather than
/// exact: an unmatched marker is left alone, so a body that merely contains an
/// asterisk still reads.
fn strip_inline_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '`' => {
                // A fenced block's ``` line goes with the inline backticks.
                let rest = &text[index..];
                if rest.starts_with("```") {
                    let skip = rest.find('\n').unwrap_or(rest.len());
                    for _ in 0..skip {
                        chars.next();
                    }
                }
            }
            '*' | '_' => {
                // Emphasis runs, of one marker or two.
                let mut run = 1;
                while chars.peek().is_some_and(|(_, next)| *next == ch) && run < 3 {
                    chars.next();
                    run += 1;
                }
            }
            '!' if text[index..].starts_with("![") => {
                // An image: skip to the closing bracket of the alt text, then past
                // the target.
                if let Some(close) = text[index..].find(']') {
                    let after = index + close + 1;
                    let skip = match text[after..].strip_prefix('(') {
                        Some(_) => text[after..].find(')').map(|end| end + 1).unwrap_or(0),
                        None => 0,
                    };
                    for _ in 0..(close + 1 + skip) {
                        chars.next();
                    }
                } else {
                    out.push(ch);
                }
            }
            '[' => {
                // A link: its label, without the target.
                if let Some(close) = text[index..].find(']') {
                    let label = &text[index + 1..index + close];
                    out.push_str(label);
                    let after = index + close + 1;
                    let skip = match text[after..].strip_prefix('(') {
                        Some(_) => text[after..].find(')').map(|end| end + 1).unwrap_or(0),
                        None => 0,
                    };
                    for _ in 0..(close + 1 + skip) {
                        chars.next();
                    }
                } else {
                    out.push(ch);
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// `useContentActions.ts`'s `checkingInstalled` → `installed` state.
fn refresh_installed(ui: &App) {
    let Some(detail) = controller().borrow().detail.clone() else {
        return;
    };
    if detail.kind != RemoteKind::Mods {
        return;
    }
    ui.global::<ContentState>()
        .set_detail_checking_installed(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::runtime::spawn(async move {
        let info = slint_content::mods::remote::check_installed(
            &instance,
            detail.platform.api(),
            &detail.id,
        )
        .await;
        let _ = weak.upgrade_in_event_loop(move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                if let Some(open) = state.detail.as_mut()
                    && open.id == detail.id
                {
                    open.installed_mods = info.mods.clone();
                }
            }
            let ui_state = ui.global::<ContentState>();
            ui_state.set_detail_checking_installed(false);
            ui_state.set_detail_installed(info.installed);
            ui_state.set_detail_installed_version(SharedString::from(
                info.mods
                    .first()
                    .and_then(|mod_info| mod_info.version.clone())
                    .unwrap_or_default(),
            ));
        });
    });
}

fn refresh_favorited(ui: &App) {
    let state = controller();
    let state = state.borrow();
    if let Some(detail) = &state.detail {
        let favorited = state.is_favorited(detail.platform, &detail.id);
        ui.global::<ContentState>().set_detail_favorited(favorited);
    }
}

// ---------------------------------------------------------------------------
// download / remove
// ---------------------------------------------------------------------------

/// Resolves the project's best file for the instance's runtime and downloads it
/// — `useContentActions.ts`'s `resolveDownloadTask` and `install`.
async fn install(instance_id: &str, detail: &OpenDetail) -> Result<(), String> {
    let runtime =
        slint_instance::get_instance_by_id(instance_id).map(|instance| instance.config.runtime);
    // `InstanceRuntime.minecraft` is a plain string; empty means unset.
    let minecraft = runtime
        .as_ref()
        .map(|runtime| runtime.minecraft.clone())
        .filter(|minecraft| !minecraft.is_empty());
    let loader = runtime
        .as_ref()
        .and_then(|runtime| runtime.mod_loader_type.as_ref())
        .map(|loader| loader.to_string().to_lowercase());

    // A modpack is not installed into the instance; the Vue downloads it into
    // the launcher's own `modpacks` folder.
    let target_dir = if detail.kind == RemoteKind::Packs {
        slint_folder::DATA_LOCATION.root.join("modpacks")
    } else {
        slint_folder::DATA_LOCATION
            .get_instance_root(instance_id)
            .join(detail.kind.folder())
    };

    let task = match detail.platform {
        Platform::Modrinth => {
            let params = slint_modrinth::ListProjectVersionsParams {
                loaders: (detail.kind.has_loaders() && loader.is_some()).then(|| {
                    serde_json::to_string(&[loader.clone().unwrap_or_default()]).unwrap_or_default()
                }),
                game_versions: minecraft
                    .clone()
                    .map(|minecraft| serde_json::to_string(&[minecraft]).unwrap_or_default()),
                featured: None,
                include_changelog: None,
            };
            let versions = slint_modrinth::list_project_versions(&detail.id, &params)
                .await
                .map_err(|error| error.to_string())?;
            let versions = versions.as_array().cloned().unwrap_or_default();
            let version = pick_modrinth_version(&versions, minecraft.as_deref(), loader.as_deref())
                .ok_or("no compatible Modrinth version")?;
            let file = version
                .get("files")
                .and_then(Value::as_array)
                .and_then(|files| {
                    files
                        .iter()
                        .find(|file| file.get("primary").and_then(Value::as_bool) == Some(true))
                        .or_else(|| files.first())
                })
                .ok_or("no downloadable file")?;
            make_task(
                file.get("url")
                    .and_then(Value::as_str)
                    .ok_or("no download url")?,
                &target_dir.join(
                    file.get("filename")
                        .and_then(Value::as_str)
                        .ok_or("no file name")?,
                ),
                file.get("size").and_then(Value::as_u64),
                file.pointer("/hashes/sha512")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                slint_download::DownloadTaskType::ModrinthMod,
            )
        }
        Platform::CurseForge => {
            let mod_id: i64 = detail
                .id
                .parse()
                .map_err(|_| "not a CurseForge mod id".to_string())?;
            let mut params = json!({});
            let object = params.as_object_mut().expect("just built");
            if let Some(minecraft) = &minecraft {
                object.insert("gameVersion".into(), json!(minecraft));
            }
            if detail.kind.has_loaders()
                && let Some(loader) = loader.as_deref().and_then(curseforge_loader_type)
            {
                object.insert("modLoaderType".into(), json!(loader));
            }
            let response = slint_curseforge::get_mod_files(mod_id, &params)
                .await
                .map_err(|error| error.to_string())?;
            let files = response
                .get("data")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let file = files
                .iter()
                .find(|file| file.get("isAvailable").and_then(Value::as_bool) == Some(true))
                .or_else(|| files.first())
                .ok_or("no compatible CurseForge file")?;
            let file_id = file.get("id").and_then(Value::as_i64).ok_or("no file id")?;
            let url = match file.get("downloadUrl").and_then(Value::as_str) {
                Some(url) if !url.is_empty() => url.to_string(),
                // `downloadUrl` is null for some files and the API has a
                // separate endpoint for those.
                _ => slint_curseforge::get_mod_file_download_url(mod_id, file_id)
                    .await
                    .map_err(|error| error.to_string())?
                    .get("data")
                    .and_then(Value::as_str)
                    .ok_or("no download url")?
                    .to_string(),
            };
            let sha1 = file
                .get("hashes")
                .and_then(Value::as_array)
                .and_then(|hashes| {
                    hashes
                        .iter()
                        .find(|hash| hash.get("algo").and_then(Value::as_i64) == Some(1))
                })
                .and_then(|hash| hash.get("value").and_then(Value::as_str))
                .map(str::to_string);
            make_task(
                &url,
                &target_dir.join(
                    file.get("fileName")
                        .and_then(Value::as_str)
                        .ok_or("no file name")?,
                ),
                file.get("fileLength").and_then(Value::as_u64),
                sha1,
                slint_download::DownloadTaskType::CurseforgeMod,
            )
        }
    };

    std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let progress = slint_download::progress::DownloadState::default();
    slint_download::download(&task, &progress)
        .await
        .map_err(|error| error.to_string())?;

    // Re-read the mods so a later list shows the new file with its metadata.
    if detail.kind == RemoteKind::Mods {
        slint_content::mods::remote::parse_mods(instance_id).await;
    }
    Ok(())
}

fn make_task(
    url: &str,
    file: &std::path::Path,
    size_bytes: Option<u64>,
    sha: Option<String>,
    task_type: slint_download::DownloadTaskType,
) -> slint_download::DownloadTask {
    slint_download::DownloadTask {
        url: url.to_string(),
        file: file.to_path_buf(),
        size_bytes,
        // A Modrinth file carries a SHA-512 and a CurseForge one a SHA-1; the
        // two lengths tell them apart.
        checksum: match sha {
            Some(sha) if sha.len() == 40 => slint_download::Checksum::Sha1(sha),
            Some(sha) => slint_download::Checksum::Sha512(sha),
            None => slint_download::Checksum::None,
        },
        task_type,
    }
}

/// `pickModrinthVersion` (`useContentActions.ts`): the first version that has a
/// file and matches the instance's loader and Minecraft version, falling back to
/// the first that has a file at all. An empty loader/version list on the
/// version means "no constraint".
fn pick_modrinth_version<'a>(
    versions: &'a [Value],
    minecraft: Option<&str>,
    loader: Option<&str>,
) -> Option<&'a Value> {
    let has_file = |version: &Value| {
        version
            .get("files")
            .and_then(Value::as_array)
            .is_some_and(|files| !files.is_empty())
    };
    let matches = |version: &Value| {
        let list_ok = |key: &str, wanted: Option<&str>| match wanted {
            None => true,
            Some(wanted) => match version.get(key).and_then(Value::as_array) {
                Some(list) if !list.is_empty() => {
                    list.iter().any(|item| item.as_str() == Some(wanted))
                }
                _ => true,
            },
        };
        has_file(version) && list_ok("game_versions", minecraft) && list_ok("loaders", loader)
    };
    versions
        .iter()
        .find(|version| matches(version))
        .or_else(|| versions.iter().find(|version| has_file(version)))
}

// ---------------------------------------------------------------------------
// icons
// ---------------------------------------------------------------------------

/// Fetches and decodes a card icon, on the background thread.
///
/// A local icon is a `data:` URL — what the `content` crate emits for a mod's,
/// a resource pack's or a save's own icon — and a remote project's is an
/// `https:` one. Blocking, which is what `crate::runtime::block_on` is for.
fn fetch_icon(url: &str) -> Option<PendingImage> {
    if url.is_empty() {
        return None;
    }
    // Already decoded for an earlier list: nothing to fetch or decode.
    if ICONS.with(|cache| cache.borrow().contains_key(url)) {
        return None;
    }
    let bytes = if let Some(data) = url.strip_prefix("data:") {
        decode_base64(data)?
    } else {
        let response = crate::runtime::block_on(slint_shared::HTTP_CLIENT.get(url).send()).ok()?;
        crate::runtime::block_on(response.bytes()).ok()?.to_vec()
    };
    let (width, height, rgba) = decode_to_rgba(&bytes)?;
    Some(PendingImage {
        key: url.to_string(),
        width,
        height,
        rgba,
    })
}

/// Builds an `Image` on the UI thread, memoised by key — a card is rebuilt on
/// every relayout, so without the cache a resize would re-make every icon on
/// screen.
fn resolve_icon(image: PendingImage) -> Option<Image> {
    resolve_image(image, &ICONS)
}

/// One image of a detail panel's gallery strip: the bitmap plus the box it
/// takes. `.gallery-item img { height: 100%; width: auto }` sizes the box by the
/// image's own aspect ratio, and Slint cannot read an image's natural size — the
/// decoded bitmap's dimensions are only known here.
fn resolve_gallery_shot(image: PendingImage) -> Option<GalleryShot> {
    resolve_gallery_shot_with(image, &ICONS)
}

fn resolve_gallery_shot_with(
    image: PendingImage,
    cache: &'static std::thread::LocalKey<RefCell<HashMap<String, Image>>>,
) -> Option<GalleryShot> {
    let ratio = if image.height == 0 {
        1.0
    } else {
        image.width as f32 / image.height as f32
    };
    resolve_image(image, cache).map(|image| GalleryShot { image, ratio })
}

fn resolve_image(
    image: PendingImage,
    cache: &'static std::thread::LocalKey<RefCell<HashMap<String, Image>>>,
) -> Option<Image> {
    if let Some(cached) = cache.with(|cache| cache.borrow().get(&image.key).cloned()) {
        return Some(cached);
    }
    let built = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
        &image.rgba,
        image.width,
        image.height,
    ));
    cache.with(|cache| {
        cache.borrow_mut().insert(image.key, built.clone());
    });
    Some(built)
}

/// `image/png;base64,AAAA…` — the part of a `data:` URL after `data:`.
///
/// The `content` crate emits two paddings — `STANDARD` for a save's icon and
/// `STANDARD_NO_PAD` for a mod's or a resource pack's — so both are accepted.
fn decode_base64(data: &str) -> Option<Vec<u8>> {
    let (meta, payload) = data.split_once(',')?;
    if !meta.contains("base64") {
        return None;
    }
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload.trim()))
        .ok()
}

/// The decoded pixels, as the background thread produces them.
fn decode_to_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some((width, height, rgba.into_raw()))
}

// ---------------------------------------------------------------------------
// category tables
// ---------------------------------------------------------------------------

/// Modrinth's mod categories (`ContentModsModrinth.vue`'s `CATEGORIES`).
/// The category tables each list offers, in the order the Vue declares them
/// (`Content{Mods,Resourcepacks,Packs}{Modrinth,Curseforge}.vue`'s `CATEGORIES`
/// / `CURSEFORGE_CATEGORIES`). A CurseForge entry is `(id, slug)`: the id is
/// what the API filters by and the slug is what names the label. The favourites
/// chip is not here — every list appends it last.
const CURSEFORGE_MOD_CATEGORIES: [(&str, &str); 16] = [
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

const MODRINTH_MOD_CATEGORIES: [&str; 19] = [
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

const CURSEFORGE_RESOURCEPACK_CATEGORIES: [(&str, &str); 19] = [
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

const MODRINTH_RESOURCEPACK_CATEGORIES: [&str; 20] = [
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

const CURSEFORGE_PACK_CATEGORIES: [(&str, &str); 18] = [
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

const MODRINTH_PACK_CATEGORIES: [&str; 10] = [
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

    /// A row re-wraps when the panel's width changes, so its height has to be
    /// computed again with it — the filter rows used to keep the height they
    /// were given for the old width, and the search panel then sat too close to
    /// (or too far from) the grid.
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

    /// The rule that was broken: the page state is filed per *list*, so a local
    /// list has none of its own and must never read a remote one's — the local
    /// mods grid used to draw curseforge's 158 pages. A page the user moved to
    /// on one list belongs to that list alone as well.
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
