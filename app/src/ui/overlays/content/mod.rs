// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The content overlays' Slint adapter: the models the panels draw, the
//! presenters that turn the use case's cards into `ContentCard`s, and the
//! callbacks on the `ContentState` / `ContentSearch` globals.
//!
//! The UI-neutral session lives in [`crate::usecases::content`] as
//! `ContentSession`, reached through `Deref`; [`cache`] is the single parsed copy
//! of the instance's four local lists. Network and disk work reports through
//! `crate::ui::services::report`; translatable text is resolved in the views.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::{Value, json};
use slint::{ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel};

use crate::slint_backend::{
    App, AppConfig, CardTag, ContentCard, ContentSearch, ContentState, DeleteSaveState, Dialogs,
    FilterChip, FilterRow, GalleryShot, GameState, MarkdownImage, MdChunk, MdItem, Navigation,
    PageButton,
};
use content::mods::{ModLoader, ResolvedMod};
use instance::InstanceRuntime;

mod body;
pub(crate) mod cache;
mod detail;
mod grid;
mod icon;
mod lists;
mod preview;
mod search;
mod wiring;
pub(crate) mod world_map;

pub(crate) use crate::usecases::content::*;
pub(crate) use body::*;
pub(crate) use cache::*;
pub(crate) use detail::*;
pub(crate) use grid::*;
pub(crate) use icon::*;
pub(crate) use lists::*;
pub(crate) use preview::*;
pub(crate) use search::*;
pub(crate) use wiring::*;

pub(crate) const PAGE_SIZE: usize = 20;
/// How many chips one page of the version carousel *steps* by; the whole track
/// is drawn and clipped, so a wider panel shows more.
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
pub(crate) const CARD_SHIFT: i32 = 4;
/// The height of a panel's header bar, which the empty-state placeholder is
/// given the room below.
pub(crate) const TITLE_BAR_HEIGHT: i32 = 52;

/// A card's height. Every card of one list has the same shape, so it comes to one
/// number per list: the info block's padding plus its lines. Getting it wrong
/// stretches the icon, which is sized to the card's height.
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
    // Three sources, in order: the buffer this pass decoded, an earlier list's
    // decode of the same url, and — only once nothing is coming — the bundled
    // fallback. A card still waiting keeps an empty box instead, because the
    // fallback texture under the spinner would be a card claiming to have an
    // icon it does not have yet.
    let decoded = card.icon.is_some();
    let icon = card
        .icon
        .and_then(resolve_icon)
        .or_else(|| card.icon_url.as_deref().and_then(cached_icon));
    let icon_loading = !decoded && card.icon_url.is_some() && icon.is_none();
    ContentCard {
        id: SharedString::from(card.id),
        link: SharedString::from(card.link),
        title: SharedString::from(card.title),
        subtitle: SharedString::from(card.subtitle),
        has_subtitle: card.has_subtitle,
        description: SharedString::from(card.description),
        tags: ModelRc::from(Rc::new(VecModel::from(tags))),
        icon: if icon_loading {
            Image::default()
        } else {
            icon.or_else(unknown_icon).unwrap_or_default()
        },
        icon_loading,
        action_kind: SharedString::from(card.action_kind),
        shows_play: card.shows_play,
        mod_disabled: card.mod_disabled,
        spawn_x: card.spawn.map_or(0, |(x, _)| x),
        spawn_z: card.spawn.map_or(0, |(_, z)| z),
        ..Default::default()
    }
}

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
    /// An `upgrade_in_event_loop` closure has to be `Send`, and an `Rc` — which
    /// is what a Slint model is — cannot cross into one. Both the callbacks and
    /// the event-loop closures reach it through [`controller`]; async tasks
    /// carry only owned values and a `Weak<App>`.
    static CONTROLLER: Rc<RefCell<ContentController>> =
        Rc::new(RefCell::new(ContentController::new()));
}

fn controller() -> Rc<RefCell<ContentController>> {
    CONTROLLER.with(Rc::clone)
}

pub(crate) struct ContentController {
    /// Kept as `VecModel`s so a relayout can rewrite a row in place; replacing
    /// the model would destroy and re-create every card element.
    saves: Rc<VecModel<ContentCard>>,
    local_mods: Rc<VecModel<ContentCard>>,
    local_resourcepacks: Rc<VecModel<ContentCard>>,
    remote: Rc<VecModel<ContentCard>>,
    /// Every filter chip's measured width, by value. Slint measures a wrapping
    /// `FlexboxLayout` at its own preferred width, so the rows are laid out from
    /// these instead.
    filter_widths: HashMap<String, f32>,
    /// Held here so a chip's width report can rewrite one row's height in place.
    filter_rows: Rc<VecModel<FilterRow>>,
    version_page: usize,
    /// Each version chip's measured width; the carousel's offset is their
    /// prefix sum.
    version_widths: Vec<f32>,
    /// The open panel's grid width and height, reported by the grid itself.
    grid_width: i32,
    panel_height: i32,
    /// Which grid is on screen, so a resize knows what to lay out again.
    open_grid: Option<Grid>,
    /// The UI-neutral session state, reached through `Deref` so `self.form` and
    /// the like still read as fields.
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
            // Only the entries that belong to an instance go. A remote icon is keyed
            // by a `cdn.modrinth.com` URL and belongs to no instance, so clearing
            // those would re-download every project icon on screen. A `data:` URL is
            // a save's or a pack's own image, read out of the instance, so those go.
            ICONS.with(|icons| {
                icons
                    .borrow_mut()
                    .retain(|url, _| !url.starts_with("data:"))
            });
            // Screenshots are read off the instance's own directory, so all of
            // them go.
            SCREENSHOTS.with(|shots| shots.borrow_mut().clear());
        }
    }

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

fn modrinth_rows(slugs: &[&str]) -> Vec<(String, String)> {
    slugs
        .iter()
        .map(|slug| (slug.to_string(), slug.to_string()))
        .collect()
}

fn curseforge_rows(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(id, slug)| (id.to_string(), slug.to_string()))
        .collect()
}
