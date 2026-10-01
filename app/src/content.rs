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
    App, AppConfig, CardTag, ContentCard, ContentSearch, ContentState, DeleteSaveState, Dialogs,
    FilterChip, FilterRow, GalleryShot, GameState, MarkdownImage, MdChunk, MdItem, PageButton,
};
use content::mods::remote::RemoteModPlatform;
use content::mods::{ModLoader, ResolvedMod};
use instance::InstanceRuntime;

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
                let _ = weak.upgrade_in_event_loop(move |ui| {
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
                let _ = weak.upgrade_in_event_loop(move |ui| {
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
                let _ = weak.upgrade_in_event_loop(move |ui| load_saves(&ui));
            });
        });
    }
    {
        // The saves grid's trash button. It opens `ConfirmDeleteSave` rather than
        // deleting, which is what `askDeleteSave` did — the dialog is the only
        // thing standing between a mis-click and a lost world.
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
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<DeleteSaveState>().set_deleting(false);
                    // The Vue caught the failure, logged it and left the dialog
                    // open (`ConfirmDeleteSave.vue`), so a save that could not be
                    // removed is still there to be retried.
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

/// The instance's loader and Minecraft version, read off `instance.toml`.
///
/// The remote lists are seeded from it (`ensureModrinthInitialized` in the Vue
/// reads it off the instance store), so it is fetched on the runtime whenever a
/// list opens rather than on the UI thread. An instance that cannot be read
/// seeds nothing, which is the empty key the seeding below compares against.
async fn instance_runtime(instance_id: &str) -> InstanceRuntime {
    instance::get_instance_by_id(instance_id)
        .await
        .map(|instance| instance.config.runtime)
        .unwrap_or_default()
}

/// `ensure…Initialized`: the loader and version the list opens on, seeded from
/// the instance's runtime. `curseForgeInitializedFor` makes it happen once per
/// instance per list — the Vue's `if (…InitializedFor === key) return` — so a
/// list that is switched away from and back is *not* re-seeded: its selections
/// lived in the component, which the `v-if` destroyed.
fn initialize_list(state: &mut ContentController, list: &str, runtime: &InstanceRuntime) {
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

fn seed_filters(state: &mut ContentController, runtime: &InstanceRuntime) {
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
                content::saves::get_all_levels(&instance).map(|levels| {
                    levels
                        .into_iter()
                        .map(|(folder, root)| (folder, content::saves::summarize_level(&root)))
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
            let icon = content::saves::get_save_icon(&instance, &folder)
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
    level: &content::saves::LevelSummary,
    icon: Option<PendingImage>,
) -> PendingCard {
    let name = level.name.clone().unwrap_or_else(|| folder.to_string());
    let cheats = level.allow_commands;
    let last_played = level.last_played;

    let mut tags: Vec<PendingTag> = Vec::new();
    if let Some(game_type) = level.game_type {
        // The four names are words the user reads, so they are resolved in the
        // card (`ContentText.save-tag`) rather than composed here: a string Rust
        // pushed is fixed at that moment and would not follow a language change.
        let kind = match game_type {
            0 => "game-mode-survival",
            1 => "game-mode-creative",
            2 => "game-mode-adventure",
            3 => "game-mode-spectator",
            _ => "",
        };
        if !kind.is_empty() {
            tags.push(translated_tag(kind));
        }
    }
    if cheats {
        tags.push(translated_tag("command-enabled"));
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
        // `ContentSaves.vue`'s `saveSpawnX` / `saveSpawnZ`, handed to
        // `WorldMap` as `:center-x` / `:center-z`. `None` where there is no
        // `Data.spawn.pos`, which is the same as `(0, 0)` here and asks the
        // world for its own spawn.
        spawn: level.spawn.map(|pos| (pos[0], pos[2])),
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

/// A tag whose *text* is a word the user reads and the card has to resolve:
/// the saves' game-mode and cheats chips. Only the kind travels, and
/// `ContentText.save-tag` turns it into the sentence in the current language.
fn translated_tag(kind: &str) -> PendingTag {
    PendingTag {
        text: String::new(),
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
        let mods = content::mods::remote::parse_mods(&instance).await;
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
            move || content::resourcepack::get_instance_resourcepacks(&instance)
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

fn resourcepack_card(pack: &content::resourcepack::Resourcepack) -> PendingCard {
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
                if let Ok(levels) = content::saves::get_all_levels(&instance) {
                    let mut folders: Vec<String> = levels.keys().cloned().collect();
                    folders.sort();
                    for folder in folders.into_iter().take(PREVIEW_ICONS) {
                        // Blocking: it reads the level's `icon.png` and encodes it.
                        let Ok(icon) = crate::runtime::block_on(content::saves::get_save_icon(
                            &instance, &folder,
                        )) else {
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
                    crate::runtime::block_on(content::mods::remote::parse_mods(&instance))
                        .iter()
                        .filter(|mod_info| !mod_info.embedded)
                        .filter_map(|mod_info| mod_info.icon.as_deref())
                        .take(PREVIEW_ICONS)
                        .filter_map(fetch_icon)
                        .collect();
                let packs: Vec<PendingImage> =
                    content::resourcepack::get_instance_resourcepacks(&instance)
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|pack| pack.icon.as_deref())
                        .take(PREVIEW_ICONS)
                        .filter_map(fetch_icon)
                        .collect();
                let shots: Vec<PendingImage> = content::screenshots::list_screenshots(&instance)
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
        let paths = content::screenshots::list_screenshots(&instance).unwrap_or_default();
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
    let params = modrinth::SearchParameters {
        query: Some(form.query.trim().to_string()).filter(|query| !query.is_empty()),
        facets: Some(serde_json::to_string(&facets).map_err(|error| error.to_string())?),
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
        "gameId": curseforge::MINECRAFT_GAME_ID,
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

    let response = curseforge::search_mods(&params)
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

/// Opens a project's detail panel from outside the content overlays — the
/// command palette's "open" on a Modrinth or CurseForge search result.
///
/// The Vue writes the id into `useShowContentDetails().value.{modrinth,curseforge}.mod`
/// and the detail component watches it; here a card click and a palette result
/// take the same `open_detail` path, so the only thing to set up first is the
/// kind and the platform. Both are the mods' whatever the project actually is:
/// the Vue has a `mod` slot per site and fills it from a search that was sent
/// without facets, so a resource pack opened this way opens the *mod* panel,
/// exactly as it does there.
pub(crate) fn open_project_detail(ui: &App, platform: &str, id: &str) {
    let platform = if platform == "curseforge" {
        Platform::CurseForge
    } else {
        Platform::Modrinth
    };
    {
        let state = controller();
        let mut state = state.borrow_mut();
        state.sync_instance(ui);
        state.platform = platform;
        state.kind = RemoteKind::Mods;
    }
    ui.global::<ContentState>()
        .set_remote_kind(SharedString::from(RemoteKind::Mods.key()));
    open_detail(ui, platform, id.to_string());
}

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
    // The body of the panel being left, not just its string: the display list
    // belongs to the document that was in the box, and a stale one would be drawn
    // under the new one's title until the new one is measured.
    clear_detail_body(ui);
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
                    ui_state.set_detail_icon(
                        loaded
                            .icon
                            .and_then(resolve_icon)
                            .or_else(unknown_icon)
                            .unwrap_or_default(),
                    );
                    ui_state.set_detail_source(SharedString::from(loaded.source_url));
                    ui_state.set_detail_source_label(SharedString::from(loaded.source_label));
                    ui_state.set_detail_source_is_github(loaded.source_is_github);
                    ui_state.set_detail_downloads(SharedString::from(loaded.downloads));
                    ui_state.set_detail_followers(SharedString::from(loaded.followers));
                    ui_state.set_detail_has_followers(loaded.has_followers);
                    ui_state.set_detail_description(SharedString::from(loaded.description));
                    // The string goes in for the record — the panel's own `v-if`
                    // reads it, and so does anything that wants to know whether
                    // there is a body at all — and the layout follows from it.
                    ui_state.set_detail_body(SharedString::from(loaded.body.as_str()));
                    set_detail_body(&ui, &loaded.body, loaded.body_is_html);
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
            // A body that is only text needs nothing fetched, and this returns
            // immediately when it has no images to fetch.
            fetch_body_images(&ui);
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
    /// Whether `body` is HTML rather than Markdown. A CurseForge summary arrives
    /// as HTML, a Modrinth body as Markdown, and the two go through different
    /// parsers.
    body_is_html: bool,
    gallery: Vec<PendingImage>,
}

async fn load_detail(platform: Platform, id: &str) -> Result<LoadedDetail, String> {
    match platform {
        Platform::Modrinth => {
            let project = modrinth::get_project(id)
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
                // The README, as the project ships it. Nothing flattens it any
                // more: the body is parsed and laid out by `slint-markdown` and
                // drawn by `markdown-body.slint`, so the markup reaches the panel
                // intact for the first time since the Vue's `v-html`.
                body: project
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                body_is_html: false,
                gallery,
            })
        }
        Platform::CurseForge => {
            let mod_id: i64 = id
                .parse()
                .map_err(|_| "not a CurseForge mod id".to_string())?;
            let response = curseforge::get_mod(mod_id)
                .await
                .map_err(|error| error.to_string())?;
            let mod_info = response.get("data").cloned().unwrap_or(Value::Null);
            let source_url = mod_info
                .pointer("/links/sourceUrl")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let description = curseforge::get_mod_description(mod_id, &json!({ "markup": true }))
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
                // A CurseForge summary arrives as HTML rather than Markdown, so
                // the body is parsed by the tag-soup front end instead. Both
                // converge on one block tree, so a heading is the same block
                // whichever platform it came from.
                body: if unsafe_html {
                    String::new()
                } else {
                    html.trim().to_string()
                },
                body_is_html: true,
                // The Vue's galleries are a Modrinth-only feature.
                gallery: Vec::new(),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// the detail panel's body
// ---------------------------------------------------------------------------

thread_local! {
    /// The Markdown engine behind the detail panel's body.
    ///
    /// A `thread_local` for the same reason [`CONTROLLER`] is one: a
    /// `fontique::Collection` is not `Send`, and the engine holds one. It is also
    /// worth keeping between panels, because the collection is a font cache and
    /// throwing it away per panel would re-read every font file on every open.
    static BODY: RefCell<Option<BodyRenderer>> = const { RefCell::new(None) };
}

struct BodyRenderer {
    renderer: markdown::Renderer,
    /// The width the display list was last laid out for, so a repeat of the same
    /// width — which a `changed width` binding produces while nothing has moved —
    /// does not re-measure the document.
    width: f32,
    /// The two models, kept rather than rebuilt per layout for the reason the
    /// grids keep theirs (`set_cards`): a document is re-laid out on every pixel
    /// of a window drag, and replacing the model would destroy and re-create
    /// every item element each time.
    /// The runs, and each run's own items and bitmaps. A `VecModel` per run rather
    /// than one for the document, because a run is what a view stacks and what a
    /// `<details>` moves: the items have to be nested inside it or the view cannot
    /// put them back where they belong. The count is the number of `<details>` in
    /// the document plus one, so these are few.
    chunks: Rc<VecModel<MdChunk>>,
    /// A run's items and bitmaps, kept beside it so a re-layout rewrites them in
    /// place: a window drag re-lays the document out, and replacing a model would
    /// destroy and re-create every item element each time.
    run_items: Vec<Rc<VecModel<MdItem>>>,
    run_images: Vec<Rc<VecModel<MarkdownImage>>>,
    /// Which runs are open, index-aligned with `chunks`, held as a model for the
    /// same reason: a layout replaces it wholesale, and a toggle writes the one
    /// row that changed.
    section_open: Rc<VecModel<bool>>,
    /// Which sections the reader has opened, by section id.
    ///
    /// Kept here rather than left to the view because a width change re-lays the
    /// document out, and that rebuilds the sections: state the view held in a
    /// property would be gone the first time the window moved a pixel, and a
    /// `<details>` that collapsed itself on a window drag is worse than one that
    /// cannot collapse at all.
    open_sections: Vec<(u32, bool)>,
    /// Whether the models have been handed to the global yet. They are created
    /// with the engine and outlive a `clear`, so a panel that is reopened finds
    /// them already bound.
    bound: bool,
}

impl BodyRenderer {
    /// An engine for a test: the platform's fonts rather than Slint's.
    ///
    /// `shared_collection()` initialises the window backend, which has to happen
    /// once and on the main thread, so a test that built one would fail for a
    /// reason that has nothing to do with the body. These tests are about the
    /// sequencing, and the family does not change whether a document is still in
    /// the engine when the width arrives.
    #[cfg(test)]
    fn for_test() -> Self {
        let mut renderer = markdown::Renderer::with_collection(markdown::fonts::system());
        renderer.set_style(
            markdown::MdStyle::default().with_families(theme_font_family(), "monospace"),
        );
        Self {
            renderer,
            width: 0.0,
            chunks: Rc::new(VecModel::default()),
            run_items: Vec::new(),
            run_images: Vec::new(),
            section_open: Rc::new(VecModel::default()),
            open_sections: Vec::new(),
            bound: false,
        }
    }

    /// A new engine, with the style `markdown-body.less` describes.
    ///
    /// The collection is Slint's own, and that is not a convenience: the engine
    /// measures with it and the panel draws with it, so two collections would be
    /// two sets of advances and every line break would be wrong. The families are
    /// named because the same has to be true of the family — the engine shapes
    /// with `Comfortaa Nunito` and the view draws with `Theme.font-family`, which
    /// is that face.
    fn new() -> Self {
        let mut renderer =
            markdown::Renderer::with_collection(slint::fontique_011::shared_collection());
        let mono = monospace_family();
        renderer.set_style(
            markdown::MdStyle::default().with_families(theme_font_family(), mono.clone()),
        );
        // The view is told the same name, and it has to be told: see
        // [`monospace_family`].
        MONO_FAMILY.with(|slot| *slot.borrow_mut() = mono);
        Self {
            renderer,
            width: 0.0,
            chunks: Rc::new(VecModel::default()),
            run_items: Vec::new(),
            run_images: Vec::new(),
            section_open: Rc::new(VecModel::default()),
            open_sections: Vec::new(),
            bound: false,
        }
    }
}

thread_local! {
    /// The monospace family the view draws code with, resolved once by
    /// [`monospace_family`].
    static MONO_FAMILY: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The monospace family, named the way *this* platform's font stack names it.
///
/// `markdown-body.less` asks CSS for `monospace` and lets the browser resolve it.
/// Slint's `font-family` is not CSS: it takes a family *name*, and the generic
/// names mean nothing to it, so a view given `"monospace"` falls back to the
/// window's default face. The engine, on the other hand, resolves
/// `GenericFamily::Monospace` and shaped with a real monospace font — so the two
/// halves measured and drew in *different faces*, which is the worst of both
/// worlds: the code came out in the body typeface, and its advances were narrower
/// than the box the engine had reserved for it, which showed as a band of empty
/// space at the right of every inline-code capsule.
///
/// Asking fontique which family it registers as the generic monospace is portable
/// and always answers with something that exists. On this machine it answers
/// *Courier*, though, which is a monospace face from 1954 and looks like it next
/// to a rounded sans — and a browser asked the same question on the same machine
/// says Menlo or SF Mono. So a short preference list is tried first, and
/// fontique's own answer is the fallback rather than the first choice.
fn monospace_family() -> String {
    use slint::fontique_011::fontique::GenericFamily;
    let mut collection = slint::fontique_011::shared_collection();

    // Best first, per platform. The names a system registers privately are
    // spelled with the leading dot they are registered under.
    #[cfg(target_os = "macos")]
    const PREFERRED: &[&str] = &[
        ".SF NS Mono",
        "SF Mono",
        "Menlo",
        "Monaco",
        "Andale Mono",
        "PT Mono",
        "Courier New",
    ];
    #[cfg(target_os = "windows")]
    const PREFERRED: &[&str] = &["Cascadia Mono", "Consolas", "Lucida Console"];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const PREFERRED: &[&str] = &[
        "DejaVu Sans Mono",
        "Liberation Mono",
        "Noto Sans Mono",
        "Ubuntu Mono",
    ];

    for name in PREFERRED {
        if collection.family_id(name).is_some() {
            return (*name).to_string();
        }
    }

    let id = collection.generic_families(GenericFamily::Monospace).next();
    id.and_then(|id| collection.family_name(id).map(str::to_owned))
        .unwrap_or_else(|| {
            log::warn!("no monospace family is registered; code will draw in the body face");
            "monospace".to_string()
        })
}

/// The monospace family the body was laid out with, for the view to draw with.
fn body_mono_family() -> String {
    MONO_FAMILY.with(|family| family.borrow().clone())
}

/// The body face, which has to be the one `Theme.font-family` names.
///
/// It is a constant (`theme.slint`) rather than a global Rust can read, and a
/// second place to change it is a second place to get wrong: a body laid out
/// with one face and drawn with another has every line the wrong length. A
/// `Theme` token that moved would have to move here too.
fn theme_font_family() -> &'static str {
    "Comfortaa Nunito"
}

/// Gives the panel a body and lays it out for the width it already has.
///
/// The view reports the width through `body-resized`, so a body set before the
/// panel has been measured waits for that; one set after — a second project
/// opened into a panel that is already on screen — is laid out at once.
fn set_detail_body(ui: &App, body: &str, is_html: bool) {
    if set_body_source(body, is_html) {
        push_detail_body(ui);
    } else {
        // The panel has not been measured yet, so there is nothing to lay the
        // document out for and nothing to draw. Only the *model* is emptied. The
        // document is not, because the width arrives a moment later — the view's
        // `changed width` — and by then the document has to still be there.
        clear_detail_body_model(ui);
    }
}

/// Opens or closes a `<details>`, by the id the engine gave it.
///
/// The id rather than an index, because the view is the one holding the array and
/// an index would only be meaningful to whoever last laid the document out. The
/// flip is applied to the bound model and *then* remembered: the model is what the
/// next layout would rebuild from, and a section that reopened by itself after a
/// window drag would be a worse bug than a lost toggle.
fn toggle_detail_section(_ui: &App, id: u32) {
    // The state lives in the bound model, so there is nothing to write through the
    // UI here: `section_open` is what the view reads, and it is updated below.
    let toggled = BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let body = slot.as_mut()?;
        // The run is named by the section id the view read off it, so the two do
        // not have to agree on an index.
        let index = (0..body.chunks.row_count()).find(|index| {
            body.chunks
                .row_data(*index)
                .is_some_and(|run| run.section_index == id as i32)
        })?;
        let open = body.section_open.row_data(index)?;
        let next = !open;
        body.section_open.set_row_data(index, next);
        // One entry per id: a second document numbers its sections from zero
        // again, and a remembered answer from the last one would open this
        // document's first section by accident.
        body.open_sections.retain(|(seen, _)| *seen != id);
        body.open_sections.push((id, next));
        Some(next)
    });
    if toggled.is_none() {
        log::warn!("a <details> was toggled that the engine no longer has: {id}");
    }
}

/// Puts a document in the engine, and reports whether there is a width to lay it
/// out for.
fn set_body_source(body: &str, is_html: bool) -> bool {
    set_body_source_with(body, is_html, BodyRenderer::new)
}

/// [`set_body_source`], with the engine's constructor handed in.
///
/// A constructor rather than a collection, because the collection itself is a
/// `fontique` type the app does not depend on directly, and because a test has
/// to be able to build an engine without the window backend — which
/// `shared_collection()` initialises, once, on the main thread.
fn set_body_source_with(body: &str, is_html: bool, new_engine: fn() -> BodyRenderer) -> bool {
    let format = if is_html {
        markdown::SourceFormat::Html
    } else {
        markdown::SourceFormat::Markdown
    };
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let engine = slot.get_or_insert_with(new_engine);
        engine.renderer.set_source(body, format);
        // Which `<details>` the reader opened belongs to the document they opened
        // it in. The engine numbers its sections from zero every time, so a
        // remembered answer would land on whatever section happens to sit at that
        // index in the *new* page.
        engine.open_sections.clear();
        engine.width > 0.0
    })
}

/// Whether the engine holds a document. Only in the tests.
#[cfg(test)]
fn body_has_document() -> bool {
    BODY.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|body| !body.renderer.blocks().is_empty())
    })
}

/// Lays the body out for a width the view reported, and pushes the result.
fn layout_detail_body(ui: &App, width: f32) {
    let changed = BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(body) = slot.as_mut() else {
            return false;
        };
        // `set_width` ignores sub-pixel changes on its own and reports whether
        // anything needs laying out again, which is also how a repeated width is
        // skipped.
        if body.renderer.set_width(width) {
            body.width = width;
            true
        } else {
            false
        }
    });
    if changed {
        push_detail_body(ui);
    }
}

/// What the engine says the panel should draw, with the UI left out of it.
///
/// The split is what makes the sequencing testable: whether a document set
/// before the panel has been measured survives until the width arrives is a
/// question about the engine and nothing else, and a function that took an `App`
/// could only be answered by opening a window.
struct BodyLayout {
    /// The runs, each with the items and bitmaps it owns.
    chunks: Vec<BodyRun>,
    /// Which runs the reader has opened, index-aligned with `chunks`. A run nobody
    /// has an answer for takes the document's own `open`, which is the
    /// `<details open>` attribute: a README's sections have none, so they start
    /// closed.
    section_open: Vec<bool>,
    /// How tall the engine laid the document out, with every section open. The
    /// panel does not use it — the box measures itself from the view, which
    /// shrinks as sections close, and the engine's height would leave a band of
    /// empty space under the document — but it is what the layout tests assert
    /// on, and what says whether the engine produced anything at all.
    height: f32,
}

/// One run, in the two forms it has to be in: the values the model row is built
/// from, and the two nested models the row holds.
struct BodyRun {
    chunk: MdChunk,
    items: Vec<MdItem>,
    images: Vec<MarkdownImage>,
}

/// Lays the engine's document out for the width it has, if it has one.
///
/// `None` until the view has reported a width. It is not the same as an empty
/// layout: an empty document is a real answer at a real width, and a document
/// nobody has measured yet has to be kept, because the width arrives a moment
/// later and by then it still has to be there.
fn body_layout() -> Option<BodyLayout> {
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let body = slot.as_mut()?;
        if body.width <= 0.0 {
            return None;
        }
        let list = body.renderer.layout();
        let mut chunks = Vec::with_capacity(list.chunks.len());
        let mut section_open = Vec::with_capacity(list.chunks.len());
        for run in &list.chunks {
            let mut items: Vec<MdItem> = Vec::with_capacity(run.items.len());
            let mut images: Vec<MarkdownImage> = Vec::new();
            for item in &list.items[run.items.clone()] {
                // A run's items are placed against the *run*, not the document:
                // the view stacks the runs and a run is what a `<details>` moves,
                // so an item's own `y` is only right relative to its own run. The
                // document's coordinates are the engine's, and stay absolute there.
                let y = item.y - run.top;
                // A bitmap is not an `MdItem`: a `slint::Image` cannot be left
                // empty in a struct literal, so an item without one would not be a
                // value the panel could build at all. The two lists share one
                // coordinate space and the view draws each in its own loop.
                if item.kind == markdown::ItemKind::Image {
                    images.push(MarkdownImage {
                        x: item.x,
                        y,
                        width: item.width,
                        height: item.height,
                        radius: item.radius,
                        alt: SharedString::from(item.text.as_str()),
                        bitmap: item.image.clone().unwrap_or_default(),
                        link_url: SharedString::from(item.link_url.as_str()),
                        link_id: SharedString::from(item.link_id.as_str()),
                    });
                    continue;
                }
                items.push(MdItem {
                    kind: SharedString::from(item.kind.name()),
                    x: item.x,
                    y,
                    width: item.width,
                    height: item.height,
                    text: SharedString::from(item.text.as_str()),
                    font_size: item.font_size,
                    font_weight: i32::from(item.font_weight),
                    font_italic: item.font_italic,
                    mono: item.mono,
                    color: SharedString::from(item.color.name()),
                    radius: item.radius,
                    filled: item.filled,
                    border_width: item.border_width,
                    border_color: SharedString::from(item.border_color.name()),
                    link_url: SharedString::from(item.link_url.as_str()),
                    link_id: SharedString::from(item.link_id.as_str()),
                    rule_offset: item.rule_offset,
                    rule: item.rule,
                });
            }
            // A run that is not a `<details>` has nothing to open, and the value
            // the array holds for it is never read — but the array is
            // index-aligned with the runs, so it has a slot either way.
            let open = match &run.section {
                None => false,
                Some(section) => body
                    .open_sections
                    .iter()
                    .find(|(id, _)| *id == section.id)
                    .map(|(_, open)| *open)
                    .unwrap_or(section.open),
            };
            section_open.push(open);
            let (
                head_x,
                head_y,
                head_width,
                head_height,
                head_radius,
                marker_x,
                marker_y,
                marker_size,
                hidden,
            ) = match &run.section {
                None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                Some(section) => (
                    section.head_x,
                    section.head_y,
                    section.head_width,
                    section.head_height,
                    section.head_radius,
                    section.marker_x,
                    section.marker_y,
                    section.marker_size,
                    section.hidden,
                ),
            };
            chunks.push(BodyRun {
                chunk: MdChunk {
                    section_index: run.section.as_ref().map_or(-1, |section| section.id as i32),
                    top: run.top,
                    height: run.height,
                    hidden,
                    head_x,
                    head_y,
                    head_width,
                    head_height,
                    head_radius,
                    marker_x,
                    marker_y,
                    marker_size,
                    // Filled in below, once the run's own models exist: a model
                    // cannot be part of the value the row is built from and part of
                    // the model at the same time.
                    items: ModelRc::default(),
                    images: ModelRc::default(),
                },
                items,
                images,
            });
        }
        Some(BodyLayout {
            chunks,
            section_open,
            height: list.height,
        })
    })
}

/// Pushes the display list into the model the panel draws.
fn push_detail_body(ui: &App) {
    let Some(BodyLayout {
        chunks,
        section_open,
        height,
    }) = body_layout()
    else {
        // The panel has not been measured, so there is nothing to lay the
        // document out for. Only the model goes: the document itself stays, for
        // the width that is about to arrive.
        clear_detail_body_model(ui);
        return;
    };
    push_detail_body_rows(ui, chunks, section_open, height);
}

/// The mechanical half of [`push_detail_body`], once there is something to push.
fn push_detail_body_rows(ui: &App, runs: Vec<BodyRun>, section_open: Vec<bool>, height: f32) {
    let state = ui.global::<ContentState>();
    // The view needs the same monospace family the engine measured with, and this
    // is the first time the engine exists — so this is the first time the answer
    // is known. Pushing it on every layout would be a no-op after the first.
    state.set_detail_body_mono_family(SharedString::from(body_mono_family()));
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(body) = slot.as_mut() else { return };
        // The first layout is also the first time the panel is shown the models,
        // and from then on they are written in place: replacing one destroys and
        // re-creates every item element, and a body is re-laid out on every pixel
        // of a window drag.
        if !body.bound {
            body.bound = true;
            state.set_detail_body_chunks(ModelRc::from(body.chunks.clone()));
            state.set_detail_body_section_open(ModelRc::from(body.section_open.clone()));
        }
        // A run's items and bitmaps are models of their own, nested inside the run
        // that holds them, so they are created once and rewritten after that.
        if body.run_items.len() != runs.len() {
            body.run_items = runs
                .iter()
                .map(|run| Rc::new(VecModel::from(run.items.clone())))
                .collect();
            body.run_images = runs
                .iter()
                .map(|run| Rc::new(VecModel::from(run.images.clone())))
                .collect();
            let rows: Vec<MdChunk> = runs
                .iter()
                .enumerate()
                .map(|(index, run)| MdChunk {
                    items: ModelRc::from(body.run_items[index].clone()),
                    images: ModelRc::from(body.run_images[index].clone()),
                    ..run.chunk.clone()
                })
                .collect();
            body.chunks.set_vec(rows);
        } else {
            for (index, run) in runs.into_iter().enumerate() {
                if body.run_items[index].row_count() != run.items.len() {
                    body.run_items[index].set_vec(run.items);
                } else {
                    for (row, item) in run.items.into_iter().enumerate() {
                        body.run_items[index].set_row_data(row, item);
                    }
                }
                if body.run_images[index].row_count() != run.images.len() {
                    body.run_images[index].set_vec(run.images);
                } else {
                    for (row, image) in run.images.into_iter().enumerate() {
                        body.run_images[index].set_row_data(row, image);
                    }
                }
                let current = body.chunks.row_data(index).unwrap_or_default();
                let items = ModelRc::from(body.run_items[index].clone());
                let images = ModelRc::from(body.run_images[index].clone());
                if current.items != items || current.images != images || current != run.chunk {
                    body.chunks.set_row_data(
                        index,
                        MdChunk {
                            items,
                            images,
                            ..run.chunk
                        },
                    );
                }
            }
        }
        if body.section_open.row_count() != section_open.len() {
            body.section_open.set_vec(section_open);
        } else {
            for (index, open) in section_open.into_iter().enumerate() {
                body.section_open.set_row_data(index, open);
            }
        }
    });
    state.set_detail_body_height(height);
}

/// Empties the two models and nothing else.
///
/// The document stays, and that is the whole point of it being a function of its
/// own: an unmeasured document has nothing to draw *yet*, and the width that lets
/// it be drawn is a moment away. Anything that empties the engine has to say so
/// itself, in [`forget_body_document`], so the two cannot be mistaken.
fn clear_body_models() {
    BODY.with(|slot| {
        if let Some(body) = slot.borrow_mut().as_mut() {
            body.chunks.set_vec(Vec::new());
            body.run_items.clear();
            body.run_images.clear();
            body.section_open.set_vec(Vec::new());
        }
    });
}

/// Empties what the panel draws, leaving the engine's document alone.
fn clear_detail_body_model(ui: &App) {
    let state = ui.global::<ContentState>();
    state.set_detail_body_hovered(SharedString::default());
    state.set_detail_body_height(0.0);
    clear_body_models();
}

/// Empties the body *and* the engine's document, for a panel that is about to
/// load another one. The display list belongs to the document that was in the
/// box, so it goes with it.
///
/// The *width* stays. It belongs to the view, not to the document, and the view
/// does not go away between panels — so clearing it would mean waiting for a
/// `changed width` that never comes, because a panel reopened at the same width
/// reports the same width, and the new body would never be laid out.
fn clear_detail_body(ui: &App) {
    clear_detail_body_model(ui);
    forget_body_document();
}

/// Throws the engine's document away, for a panel that is about to show another
/// one.
///
/// The *width* stays. It belongs to the view, and the view does not go away
/// between panels — so clearing it would mean waiting for a `changed width` that
/// never comes, because a panel reopened at the width it already had reports the
/// same width, and the new body would never be laid out.
fn forget_body_document() {
    BODY.with(|slot| {
        if let Some(body) = slot.borrow_mut().as_mut() {
            body.renderer
                .set_source(String::new(), markdown::SourceFormat::Markdown);
        }
    });
}

/// Fetches whatever the body is still missing, then lays it out again.
///
/// An image changes a document's height, because the engine reserves a box from
/// the bitmap's own size, so a body whose images arrive late is re-laid out once
/// per batch. The fetches are the same `fetch_icon` path the project icons take,
/// including its cache, so a logo and a README that show the same image decode it
/// once.
fn fetch_body_images(ui: &App) {
    let wanted = BODY.with(|slot| {
        let slot = slot.borrow();
        let Some(body) = slot.as_ref() else {
            return Vec::new();
        };
        body.renderer
            .referenced_images()
            .into_iter()
            .filter(|url| !body.renderer.images().contains(url))
            .collect::<Vec<_>>()
    });
    if wanted.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    crate::runtime::spawn_blocking(move || {
        let fetched = wanted
            .iter()
            .filter_map(|url| fetch_icon(url).map(|image| (url.clone(), image)))
            .collect::<Vec<_>>();
        if fetched.is_empty() {
            return;
        }
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let mut changed = false;
            for (url, image) in &fetched {
                changed |= BODY.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    match slot.as_mut() {
                        // The engine is not `Send` and the bitmap is not either, so
                        // the `Image` is built on this side of the hop, from the
                        // buffer that did cross.
                        Some(body) => body.renderer.set_image(
                            url,
                            Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
                                &image.rgba,
                                image.width,
                                image.height,
                            )),
                            image.width,
                            image.height,
                        ),
                        None => false,
                    }
                });
            }
            if changed {
                push_detail_body(&ui);
            }
        });
    });
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
        let info =
            content::mods::remote::check_installed(&instance, detail.platform.api(), &detail.id)
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
    let runtime = instance::get_instance_by_id(instance_id)
        .await
        .map(|instance| instance.config.runtime);
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
        folder::DATA_LOCATION.root.join("modpacks")
    } else {
        folder::DATA_LOCATION
            .get_instance_root(instance_id)
            .join(detail.kind.folder())
    };

    let task = match detail.platform {
        Platform::Modrinth => {
            let params = modrinth::ListProjectVersionsParams {
                loaders: (detail.kind.has_loaders() && loader.is_some()).then(|| {
                    serde_json::to_string(&[loader.clone().unwrap_or_default()]).unwrap_or_default()
                }),
                game_versions: minecraft
                    .clone()
                    .map(|minecraft| serde_json::to_string(&[minecraft]).unwrap_or_default()),
                featured: None,
                include_changelog: None,
            };
            let versions = modrinth::list_project_versions(&detail.id, &params)
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
                download::DownloadTaskType::ModrinthMod,
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
            let response = curseforge::get_mod_files(mod_id, &params)
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
                _ => curseforge::get_mod_file_download_url(mod_id, file_id)
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
                download::DownloadTaskType::CurseforgeMod,
            )
        }
    };

    std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let progress = download::progress::DownloadState::default();
    download::download(&task, &progress)
        .await
        .map_err(|error| error.to_string())?;

    // Re-read the mods so a later list shows the new file with its metadata.
    if detail.kind == RemoteKind::Mods {
        content::mods::remote::parse_mods(instance_id).await;
    }
    Ok(())
}

fn make_task(
    url: &str,
    file: &std::path::Path,
    size_bytes: Option<u64>,
    sha: Option<String>,
    task_type: download::DownloadTaskType,
) -> download::DownloadTask {
    download::DownloadTask {
        url: url.to_string(),
        file: file.to_path_buf(),
        size_bytes,
        // A Modrinth file carries a SHA-512 and a CurseForge one a SHA-1; the
        // two lengths tell them apart.
        checksum: match sha {
            Some(sha) if sha.len() == 40 => download::Checksum::Sha1(sha),
            Some(sha) => download::Checksum::Sha512(sha),
            None => download::Checksum::None,
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
pub(crate) fn fetch_icon(url: &str) -> Option<PendingImage> {
    let bytes = crate::runtime::block_on(fetch_icon_bytes(url))?;
    decode_icon(url, bytes)
}

/// The icon's bytes, and nothing else.
///
/// Split out of [`fetch_icon`] so a caller that wants the icons of a whole page
/// at once can *await* the network and leave only the decode to a blocking
/// thread. Downloading twenty of them one after another on one thread is what
/// makes a list of remote results look dead while its images arrive.
pub(crate) async fn fetch_icon_bytes(url: &str) -> Option<Vec<u8>> {
    if url.is_empty() {
        return None;
    }
    // Already decoded for an earlier list: nothing to fetch.
    if ICONS.with(|cache| cache.borrow().contains_key(url)) {
        return None;
    }
    if let Some(data) = url.strip_prefix("data:") {
        return decode_base64(data);
    }
    let response = shared::HTTP_CLIENT.get(url).send().await.ok()?;
    Some(response.bytes().await.ok()?.to_vec())
}

/// Decodes what [`fetch_icon_bytes`] brought back. A local icon and a remote
/// project's differ only in how the bytes were obtained.
pub(crate) fn decode_icon(url: &str, bytes: Vec<u8>) -> Option<PendingImage> {
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
pub(crate) fn resolve_icon(image: PendingImage) -> Option<Image> {
    resolve_image(image, &ICONS)
}

/// The card icon a save, mod, resource pack or pack falls back to — the
/// `v-else` branch every content card in the Vue carried, which pointed at
/// `Unknown_server.webp` (now `app/ui/assets/images/unknown-server.webp`). A
/// card whose icon is missing or
/// failed to decode used to come out as an empty 72x72 box here.
///
/// Decoded on first use and kept: it is the same 120x120 bitmap every time, and
/// a card is rebuilt on every relayout.
pub(crate) fn unknown_icon() -> Option<Image> {
    if let Some(image) = UNKNOWN_ICON.with(|cached| cached.borrow().clone()) {
        return Some(image);
    }
    let bytes = include_bytes!("../ui/assets/images/unknown-server.webp");
    let (width, height, rgba) = decode_to_rgba(bytes)?;
    let image = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&rgba, width, height));
    UNKNOWN_ICON.with(|cached| *cached.borrow_mut() = Some(image.clone()));
    Some(image)
}

/// The icon at `url`, if some earlier list already fetched and decoded it.
///
/// `fetch_icon` reports a cache hit by returning nothing, so a caller that wants
/// the icon either way — the command palette redraws its results on every
/// keystroke — asks for the cache when the fetch came back empty.
pub(crate) fn cached_icon(url: &str) -> Option<Image> {
    if url.is_empty() {
        return None;
    }
    ICONS.with(|cache| cache.borrow().get(url).cloned())
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

    /// The body state, with nothing in it. Every test here drives the same
    /// `thread_local` the panel does, so they are ordered against nothing and
    /// have to clean up after themselves.
    fn reset_body() {
        BODY.with(|slot| *slot.borrow_mut() = None);
    }

    /// [`set_body_source`], with an engine that needs no window backend.
    fn set_source(body: &str) -> bool {
        set_body_source_with(body, false, BodyRenderer::for_test)
    }

    /// An engine that already knows its width, which is the state a panel is in
    /// once its view has reported one.
    fn measured_body() {
        BODY.with(|slot| {
            let mut slot = slot.borrow_mut();
            let engine = slot.get_or_insert_with(BodyRenderer::for_test);
            engine.width = 600.0;
            engine.renderer.set_width(600.0);
        });
    }

    #[test]
    fn a_body_set_before_the_panel_is_measured_survives_until_the_width_arrives() {
        // The bug this is here for. Setting a document and then finding no width
        // used to go through the "empty the body" path, which emptied the engine
        // too -- so the width arrived a moment later, by which point the engine
        // held an empty string and laid *that* out. The panel drew nothing, for a
        // body that had arrived perfectly intact.
        reset_body();

        assert!(!set_source("# A heading\n\nA paragraph.\n"));
        assert!(body_has_document(), "the document arrived");

        // No width, so nothing to draw. The model goes; the document does not.
        clear_body_models();
        assert!(
            body_has_document(),
            "emptying what is drawn must not empty the engine"
        );
        assert!(
            body_layout().is_none(),
            "an unmeasured document has no layout, and that is not a failure"
        );

        // Then the view reports its width, and the document is still there.
        measured_body();
        let layout = body_layout().expect("a layout once the width is known");
        assert!(!layout.chunks.is_empty(), "the document was lost");
        assert!(layout.height > 0.0, "a document with text has a height");
        reset_body();
    }

    #[test]
    fn a_body_keeps_its_width_across_panels() {
        // The width belongs to the view, not to the document. A panel reopened at
        // the width it already had reports the same width, so `changed width`
        // never fires -- and a body that waited for one would never be laid out.
        reset_body();
        measured_body();
        assert!(set_source("first"));
        assert!(body_layout().is_some(), "the first body lays out");

        // A second panel: the document is cleared, the width is not.
        forget_body_document();
        assert!(!body_has_document());
        BODY.with(|slot| {
            let body = slot.borrow();
            let body = body.as_ref().expect("the engine");
            assert_eq!(body.width, 600.0, "the width went with the document");
        });

        // And the new body is laid out at once, without waiting for a width that
        // is not coming.
        assert!(set_source("second"), "the width survived");
        let layout = body_layout().expect("the second body lays out at once");
        assert!(!layout.chunks.is_empty(), "the second document was lost");
        reset_body();
    }

    #[test]
    fn an_empty_document_is_a_layout_of_nothing_rather_than_no_layout() {
        // The distinction that made the first test's bug invisible: both answer
        // "there is nothing to draw", and only one of them is a reason to keep
        // the document.
        reset_body();
        measured_body();
        forget_body_document();
        let layout = body_layout().expect("a width is a width, empty or not");
        assert!(layout.chunks.is_empty());
        assert_eq!(layout.height, 0.0);
        reset_body();
    }

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
