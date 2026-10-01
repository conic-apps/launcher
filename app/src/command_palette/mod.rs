// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The command palette (`src/overlays/CommandPalette.vue`).
//!
//! Three modes, one list. The root mode offers the five fixed commands and —
//! when the query matches none of them — the instance list; "launch-instance"
//! offers the instances alone; "search-online" offers whatever Modrinth or
//! CurseForge has for the keyword. Every mode draws the same row and the same
//! footer, and the mode picks the placeholder, the breadcrumb and which of the
//! two footer hints is shown.
//!
//! Two conventions it follows from the rest of the port:
//!
//!   * The rendered list is built here rather than in an expression. Slint can
//!     neither build a model nor index one, and the list's order, its filtering,
//!     its section headings and every row's height have to agree — which is the
//!     same reason `game.rs` builds `GameRow`s and `content.rs` lays the card
//!     grid out.
//!   * Nothing here composes a *translatable* string. A string pushed into a
//!     model is fixed at the moment it is pushed and would not follow a language
//!     change, where an `@tr` binding re-evaluates, so a row carries the *key* of
//!     its label and `CommandText` resolves it
//!     (`ui/globals/command-palette.slint`).
//!
//! The two searches run on the tokio runtime and report back through
//! `upgrade_in_event_loop`, exactly as the content overlays' do.

use std::cell::RefCell;
use std::rc::Rc;

use instance::{Instance, SortBy};
use serde_json::Value;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::content::{self, PendingImage};
use crate::slint_backend::{App, CommandPaletteState, Dialogs, GameState, Navigation, PaletteItem};

mod search;
mod wiring;

pub(crate) use search::*;
pub(crate) use wiring::*;

thread_local! {
    /// The one state.
    ///
    /// It lives here rather than in a value `setup` owns because an
    /// `upgrade_in_event_loop` closure has to be `Send`, and an `Rc` — which is
    /// what a Slint model is — cannot cross into one. Slint's callbacks and the
    /// event-loop closures both run on the UI thread, so both reach it through
    /// [`controller`]; the async task carries only owned values and a `Weak<App>`.
    static CONTROLLER: Rc<RefCell<PaletteController>> =
        Rc::new(RefCell::new(PaletteController::new()));
}

/// The controller, for use on the UI thread.
pub(crate) fn controller() -> Rc<RefCell<PaletteController>> {
    CONTROLLER.with(Rc::clone)
}

// ---------------------------------------------------------------------------
// geometry, from the stylesheet
// ---------------------------------------------------------------------------

/// `.section-label`'s own box — `padding: 6px 10px 2px` around a `line-height: 1`
/// 11px line — plus 4px more, so the heading does not sit flush on the row under
/// it. The Vue puts no gap here; this one is a deliberate change, and it is why
/// the constant is not just the sum of that padding and that line.
pub(crate) const LABEL_HEIGHT: f32 = 23.0;
/// `.palette-item` with no loader tags: `padding: 8px 10px` around the 24px
/// icon box, which is taller than the 13px title line on its own.
pub(crate) const ROW_HEIGHT: f32 = 40.0;
/// The same row with the tag line: 13px of title, a `gap: 3px` and a 14px tag
/// (`font-size: 10px` + `padding: 2px 5px`) make `.item-main` 30px, and 30 > 24
/// is what decides the row.
pub(crate) const ROW_HEIGHT_TAGGED: f32 = 46.0;

/// How many hits a search asks for: `limit: 20` / `pageSize: 20` in the Vue.
pub(crate) const SEARCH_LIMIT: usize = 20;

/// How long one project icon may take before the row keeps the site's mark.
///
/// The shared client has no timeout of its own, so a CDN that accepts the
/// connection and then says nothing would hold the icon pass open indefinitely —
/// and with it the next search's.
pub(crate) const ICON_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

// ---------------------------------------------------------------------------
// the five fixed commands
// ---------------------------------------------------------------------------

/// One of the root mode's five commands.
///
/// `title` is the source string, which is what the *filter* matches against. The
/// row shows `CommandText.title(title_kind)`, the translated form; see `build`
/// for why the two can differ.
pub(crate) struct Command {
    key: &'static str,
    title_kind: &'static str,
    title: &'static str,
    icon: &'static str,
    has_children: bool,
    action: Action,
}

pub(crate) const COMMANDS: [Command; 5] = [
    Command {
        key: "launch-instance",
        title_kind: "launch-instance",
        title: "Launch instance",
        icon: "play",
        has_children: true,
        action: Action::EnterMode {
            kind: "launch-instance",
            source: "",
        },
    },
    Command {
        key: "create-instance",
        title_kind: "create-instance",
        title: "Create instance",
        icon: "add",
        has_children: false,
        action: Action::CreateInstance,
    },
    Command {
        key: "add-account",
        title_kind: "add-account",
        title: "Add game profile",
        icon: "user-add",
        has_children: false,
        action: Action::AddAccount,
    },
    Command {
        key: "search-modrinth",
        title_kind: "search-modrinth",
        title: "Search on Modrinth",
        icon: "modrinth",
        has_children: true,
        action: Action::EnterMode {
            kind: "search-online",
            source: "modrinth",
        },
    },
    Command {
        key: "search-curseforge",
        title_kind: "search-curseforge",
        title: "Search on CurseForge",
        icon: "curseforge",
        has_children: true,
        action: Action::EnterMode {
            kind: "search-online",
            source: "curseforge",
        },
    },
];

// ---------------------------------------------------------------------------
// state
// ---------------------------------------------------------------------------

/// What a row does when it is chosen. Resolved when the list is built rather
/// than from the key on the way out, so a callback carrying only an index can
/// find the thing behind it — the arrangement `content.rs`'s `CardTarget` uses.
#[derive(Clone, Default)]
pub(crate) enum Action {
    /// A row with no action: only ever a blank `PendingItem` mid-build.
    #[default]
    None,
    /// A command that switches the palette to another mode. `source` is only
    /// there for `search-online`.
    EnterMode {
        kind: &'static str,
        source: &'static str,
    },
    /// `dialogStore.createInstance.visible = true`.
    CreateInstance,
    /// `dialogStore.accountAdd.visible = true`.
    AddAccount,
    /// `instanceStore.currentInstance = instance`, and the launch page when the
    /// row came out of the "launch instance" mode.
    SelectInstance { id: String, launch: bool },
    /// The game page, then the project's detail panel.
    OpenProject { platform: &'static str, id: String },
}

/// A row as it is built, before it becomes a `PaletteItem`.
///
/// A `PaletteItem` holds a `ModelRc` (`loaders`) and an `Image`, and neither
/// belongs in an `upgrade_in_event_loop` closure, so the search results cross as
/// plain data and are finished here, on the UI thread.
#[derive(Clone, Default)]
pub(crate) struct PendingItem {
    key: String,
    title: String,
    title_kind: String,
    author: String,
    loaders: Vec<String>,
    subtitle: String,
    subtitle_kind: String,
    icon: String,
    icon_is_brand: bool,
    icon_url: String,
    /// The icon as it came off the network, when it was fetched for this row.
    icon_image: Option<PendingImage>,
    has_children: bool,
    action_kind: &'static str,
    section_kind: &'static str,
    action: Action,
    /// The row's offset from the top of the list, heading included. Set by
    /// `build`, and only read for the selected row's reveal — the rows
    /// themselves are placed by the list's own layout.
    row_y: f32,
}

impl PendingItem {
    /// The height of the heading above this row, or none.
    fn label_height(&self) -> f32 {
        if self.section_kind.is_empty() {
            0.0
        } else {
            LABEL_HEIGHT
        }
    }

    /// The height of the row itself.
    fn row_height(&self) -> f32 {
        if self.loaders.is_empty() {
            ROW_HEIGHT
        } else {
            ROW_HEIGHT_TAGGED
        }
    }

    /// The Slint row. Runs on the UI thread, which is the only place a model may
    /// be made.
    fn finish(&self) -> PaletteItem {
        PaletteItem {
            key: SharedString::from(self.key.as_str()),
            title: SharedString::from(self.title.as_str()),
            title_kind: SharedString::from(self.title_kind.as_str()),
            author: SharedString::from(self.author.as_str()),
            loaders: ModelRc::from(Rc::new(VecModel::from(
                self.loaders
                    .iter()
                    .map(|loader| SharedString::from(loader.as_str()))
                    .collect::<Vec<_>>(),
            ))),
            subtitle: SharedString::from(self.subtitle.as_str()),
            subtitle_kind: SharedString::from(self.subtitle_kind.as_str()),
            icon: SharedString::from(self.icon.as_str()),
            icon_is_brand: self.icon_is_brand,
            // The `<img>` is drawn whenever the search returned a URL, whether or
            // not the bitmap arrived; the empty image is that `<img>`'s broken
            // state.
            icon_image: self.icon(),
            has_icon_image: !self.icon_url.is_empty(),
            has_children: self.has_children,
            action_kind: SharedString::from(self.action_kind),
            section_kind: SharedString::from(self.section_kind),
            label_height: self.label_height(),
            row_height: self.row_height(),
        }
    }

    /// The row's icon: the one just decoded, or the one an earlier list already
    /// decoded. `fetch_icon` reports a cache hit by returning nothing, and a
    /// project icon is the same bitmap wherever it is shown, so the content
    /// overlays' cache answers for it. A project with no icon at all gets the
    /// same unknown world a content card would.
    fn icon(&self) -> slint::Image {
        match &self.icon_image {
            Some(pending) => content::resolve_icon(pending.clone())
                .or_else(content::unknown_icon)
                .unwrap_or_default(),
            None => content::cached_icon(&self.icon_url)
                .or_else(content::unknown_icon)
                .unwrap_or_default(),
        }
    }
}

/// A project as the search returned it, before it becomes a row.
pub(crate) struct RemoteResult {
    key: String,
    title: String,
    author: String,
    icon_url: String,
    loaders: Vec<String>,
    /// "" when the site says nothing the palette has a word for.
    subtitle_kind: &'static str,
    platform: &'static str,
    id: String,
}

impl RemoteResult {
    fn to_item(&self, icon: Option<PendingImage>) -> PendingItem {
        PendingItem {
            key: self.key.clone(),
            title: self.title.clone(),
            author: self.author.clone(),
            loaders: self.loaders.clone(),
            subtitle_kind: self.subtitle_kind.to_string(),
            // The site's own mark stands in for the project icon when the search
            // returned none, which is the Vue's `v-else-if` chain: an `<img>`,
            // then an `AppIcon`, then the brand component.
            icon: self.platform.to_string(),
            icon_is_brand: true,
            icon_url: self.icon_url.clone(),
            icon_image: icon,
            action_kind: "open",
            action: Action::OpenProject {
                platform: self.platform,
                id: self.id.clone(),
            },
            ..Default::default()
        }
    }
}

pub(crate) struct PaletteController {
    /// The instance list, read from the crate when the palette opens.
    instances: Vec<Instance>,
    /// The rows on screen, and where the selection is in them.
    items: Vec<PendingItem>,
    selected: usize,
    /// The key last published as the selection. The Vue's `watch(selectedIndex)`
    /// does not fire when a `mousemove` reports the index the row already had,
    /// and a reveal on every mouse move would drag the list along with the
    /// pointer — so the sequence the view watches is bumped on a move only.
    selected_key: String,
    selected_seq: u64,
    items_seq: u64,
    results: Vec<PendingItem>,
    searching: bool,
    error: bool,
    /// Bumped on every search, so an answer for a request the user has typed past
    /// is dropped — the Vue's `onlineSearchToken`.
    token: u64,
    mode: &'static str,
    source: &'static str,
}

impl PaletteController {
    fn new() -> Self {
        Self {
            instances: Vec::new(),
            items: Vec::new(),
            selected: 0,
            selected_key: String::new(),
            selected_seq: 0,
            items_seq: 0,
            results: Vec::new(),
            searching: false,
            error: false,
            token: 0,
            mode: "root",
            source: "modrinth",
        }
    }

    // ----- opening and closing -----

    /// The Vue's `watch(() => props.visible)`: everything is reset on the way
    /// in, never on the way out.
    ///
    /// The instance list is read on the runtime and the reset happens in the
    /// event loop behind it, so opening the palette does not wait on a read of
    /// every `instance.toml` (the original read it in a Tauri command, off the
    /// UI thread, for the same reason).
    fn open(ui: &App) {
        let weak = ui.as_weak();
        crate::runtime::spawn(async move {
            // The list is read from the crate rather than borrowed from
            // `game.rs`'s rows, which are the *grouped, filtered* model the game
            // view draws and not the instance list the palette filters. What it
            // comes back sorted by does not matter: `filtered` sorts by last
            // played itself.
            let instances = instance::list_instances(SortBy::Playtime)
                .await
                .unwrap_or_default();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                controller().borrow_mut().open_with(&ui, instances);
            });
        });
    }

    /// The reset [`PaletteController::open`] performs, for a list already read.
    fn open_with(&mut self, ui: &App, instances: Vec<Instance>) {
        self.instances = instances;
        self.mode = "root";
        self.source = "modrinth";
        self.results.clear();
        self.searching = false;
        self.error = false;
        self.token += 1;
        let state = ui.global::<CommandPaletteState>();
        state.set_query(SharedString::new());
        state.set_mode(SharedString::from("root"));
        state.set_source(SharedString::from("modrinth"));
        state.set_hovered_key(SharedString::new());
        state.set_online_searching(false);
        state.set_online_error(false);
        state.set_visible(true);
        self.build(ui);
    }

    fn close(&self, ui: &App) {
        ui.global::<CommandPaletteState>().set_visible(false);
    }

    /// The `Ctrl`/`⌘` + `/` shortcut: `commandPaletteVisible.value =
    /// !commandPaletteVisible.value`.
    fn toggle(&mut self, ui: &App) {
        if ui.global::<CommandPaletteState>().get_visible() {
            self.close(ui);
        } else {
            Self::open(ui);
        }
    }

    /// `enterMode`: the mode changes and the query is emptied, which puts the
    /// search back to nothing as well.
    fn enter_mode(&mut self, ui: &App, kind: &str, source: &str) {
        self.mode = match kind {
            "launch-instance" => "launch-instance",
            "search-online" => "search-online",
            _ => "root",
        };
        self.source = if source == "curseforge" {
            "curseforge"
        } else {
            "modrinth"
        };
        self.reset_search(ui);
        let state = ui.global::<CommandPaletteState>();
        state.set_query(SharedString::new());
        // The mode and the source are published, not just kept: the view reads
        // them for the placeholder, the breadcrumb, which footer hint is drawn and
        // what Escape and Backspace do, and it was still on "root" while a submode
        // was open — so Escape closed the panel from inside one, and the search
        // field kept the root placeholder while a search ran.
        state.set_mode(SharedString::from(self.mode));
        state.set_source(SharedString::from(self.source));
        self.build(ui);
    }

    /// `backToRoot`.
    fn back_to_root(&mut self, ui: &App) {
        self.enter_mode(ui, "root", self.source);
    }

    // ----- the list -----

    /// Rebuilds the model from the mode, the query and the results.
    fn build(&mut self, ui: &App) {
        self.publish(ui, true)
    }

    /// Puts the rows on screen.
    ///
    /// `reset_scroll` is the Vue's `scrollViewRef.scrollTo(0, false)`, which
    /// every one of its `query` / `mode` / `onlineItems` watchers does — but not
    /// what refilling the icons should do, since no row has moved. Re-running it
    /// there would throw the reader back to the top every time an image landed.
    fn publish(&mut self, ui: &App, reset_scroll: bool) {
        let state = ui.global::<CommandPaletteState>();
        // `normalizedQuery`: `query.value.trim().toLowerCase()`.
        let query = state.get_query().trim().to_lowercase();
        state.set_query_empty(state.get_query().trim().is_empty());

        let mut items: Vec<PendingItem> = match self.mode {
            "launch-instance" => self
                .filtered(&query)
                .into_iter()
                .map(|instance| instance_item(instance, "launch"))
                .collect(),
            "search-online" => self
                .results
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, mut item)| {
                    // `withSectionLabel(onlineItems, …)` gives the heading to the
                    // first row alone.
                    item.section_kind = if index == 0 { "results" } else { "" };
                    item
                })
                .collect(),
            _ => {
                // The root mode's own filter: the commands whose title contains
                // the query, or — when none of them does — the instances.
                //
                // The title it matches is the *source* string rather than the
                // translated one, and that is the one deviation from the Vue: it
                // filters on `command.title`, and `title` is whatever `t()`
                // returned. A model cannot be filtered in an expression (Slint can
                // neither build a model nor index one), so the list has to be
                // built here, and the translated titles do not exist here. The
                // row still shows the translation, because that is resolved in
                // the view.
                let matched: Vec<&Command> = COMMANDS
                    .iter()
                    .filter(|command| {
                        query.is_empty() || command.title.to_lowercase().contains(&query)
                    })
                    .collect();
                if matched.is_empty() {
                    self.filtered(&query)
                        .into_iter()
                        .enumerate()
                        .map(|(index, instance)| {
                            let mut item = instance_item(instance, "select");
                            item.section_kind = if index == 0 { "instances" } else { "" };
                            item
                        })
                        .collect()
                } else {
                    matched
                        .iter()
                        .enumerate()
                        .map(|(index, command)| command_item(command, index == 0))
                        .collect()
                }
            }
        };

        // Each row's offset, and the height of the whole list, so the view can
        // place the rows, cap the results area and reveal the selection.
        let mut offset = 0.0;
        for item in &mut items {
            item.row_y = offset;
            offset += item.label_height() + item.row_height();
        }

        self.items = items;
        if self.selected >= self.items.len() {
            self.selected = 0;
        }
        state.set_items(ModelRc::from(Rc::new(VecModel::from(
            self.items
                .iter()
                .map(PendingItem::finish)
                .collect::<Vec<PaletteItem>>(),
        ))));
        state.set_items_content_height(offset);
        if reset_scroll {
            self.items_seq += 1;
            state.set_items_seq(self.items_seq as i32);
        }
        self.push_selection(ui);
    }

    /// Puts the icons that arrived into the rows that are already on screen,
    /// each at the row it was fetched for.
    fn attach_icons(&mut self, ui: &App, icons: Vec<(usize, Option<PendingImage>)>) {
        for (index, icon) in icons {
            if let Some(item) = self.results.get_mut(index) {
                item.icon_image = icon;
            }
        }
        self.publish(ui, false);
    }

    /// `filteredInstances`: the instances whose name, Minecraft version or
    /// loader contains the query, most recently played first.
    ///
    /// The Vue's comparator is `b.last_played - a.last_played`, and
    /// `last_played` is null for an instance that has never been played — a
    /// subtraction that is then `NaN`, which every engine reads as "keep the
    /// order" and which leaves those instances wherever the store's own sort put
    /// them. Comparing the `Option` puts them last instead, which is the order
    /// the expression gives for every instance that *has* been played.
    fn filtered(&self, query: &str) -> Vec<&Instance> {
        let mut instances: Vec<&Instance> = self
            .instances
            .iter()
            .filter(|instance| {
                query.is_empty()
                    || [
                        instance.config.name.as_str(),
                        instance.config.runtime.minecraft.as_str(),
                        // `mod_loader_type ?? "vanilla"`.
                        loader_name(instance).as_str(),
                    ]
                    .iter()
                    .any(|field| field.to_lowercase().contains(query))
            })
            .collect();
        instances.sort_by_key(|instance| std::cmp::Reverse(instance.last_played));
        instances
    }

    // ----- the selection -----

    fn move_selection(&mut self, ui: &App, delta: i32) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        // `(selected + 1) % count` and `(selected - 1 + count) % count`.
        let next = (self.selected as i32 + delta).rem_euclid(count as i32) as usize;
        self.select(ui, next);
    }

    /// The row the pointer is over, which is *not* the selection.
    ///
    /// The Vue moved the selection with `@mousemove` and had no hover of its own;
    /// this separates the two, so the keyboard's selection stays where it was put
    /// and the pointer only says where it is. `None` is "no row" — the pointer is
    /// over the panel rather than over one.
    fn hover(&mut self, ui: &App, index: Option<usize>) {
        let key = index
            .and_then(|index| self.items.get(index))
            .map(|item| item.key.clone())
            .unwrap_or_default();
        ui.global::<CommandPaletteState>()
            .set_hovered_key(SharedString::from(key.as_str()));
    }

    fn select(&mut self, ui: &App, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.selected = index;
        self.push_selection(ui);
    }

    /// Publishes the selected row: its key (what the view highlights), its action
    /// (what the footer shows) and its box (what the view scrolls to).
    fn push_selection(&mut self, ui: &App) {
        let item = self.items.get(self.selected);
        let key = item.map(|item| item.key.clone()).unwrap_or_default();
        let moved = key != self.selected_key;
        let state = ui.global::<CommandPaletteState>();
        state.set_selected_key(SharedString::from(key.as_str()));
        state.set_selected_action_kind(SharedString::from(
            item.map(|item| item.action_kind).unwrap_or_default(),
        ));
        state.set_selected_row_y(item.map(|item| item.row_y).unwrap_or_default());
        state.set_selected_row_height(item.map(|item| item.row_height()).unwrap_or_default());
        self.selected_key = key;
        if moved {
            self.selected_seq += 1;
            state.set_selected_seq(self.selected_seq as i32);
        }
    }

    // ----- performing a row -----

    /// A click on a row. This is *not* `performItem` and it is a deliberate
    /// departure from the Vue, which opened on the first click: a click on a row
    /// that is not the selection moves the selection, and a click on the row that
    /// already is the selection opens it. Enter opens the selection outright, so a
    /// pointer and the keyboard reach the same row by the same two steps.
    fn click(&mut self, ui: &App, index: usize) {
        if index >= self.items.len() {
            return;
        }
        let selected = self.selected == index;
        if !selected {
            self.select(ui, index);
            return;
        }
        self.perform(ui, index);
    }

    /// `performItem(item)` for the row at `index`.
    fn perform(&mut self, ui: &App, index: usize) {
        if index < self.items.len() {
            self.select(ui, index);
        }
        let Some(action) = self.items.get(index).map(|item| item.action.clone()) else {
            return;
        };
        match action {
            Action::None => {}
            Action::EnterMode { kind, source } => self.enter_mode(ui, kind, source),
            Action::CreateInstance => {
                self.close(ui);
                ui.global::<Dialogs>().set_create_instance_visible(true);
            }
            Action::AddAccount => {
                self.close(ui);
                ui.global::<Dialogs>().set_account_add_visible(true);
            }
            Action::SelectInstance { id, launch } => {
                // The Vue assigns `instanceStore.currentInstance` *before* it
                // closes, so the background has already been told by the time the
                // palette is gone.
                ui.global::<GameState>().invoke_select_instance(id.into());
                self.close(ui);
                if launch {
                    ui.global::<Navigation>().invoke_navigate("launch".into());
                }
            }
            Action::OpenProject { platform, id } => {
                self.close(ui);
                // The detail panels are mounted by the game view, so the page has
                // to be there before the id is set — the Vue's
                // `if (navigationStore.currentPage !== "game") navigate("game")`.
                if ui.global::<Navigation>().get_current_page() != "game" {
                    ui.global::<Navigation>().invoke_navigate("game".into());
                }
                content::open_project_detail(ui, platform, &id);
            }
        }
    }

    // ----- the online search -----

    /// The Vue's `watch([query, mode])` half that is not the search itself: the
    /// selection goes back to the first row.
    fn query_changed(&mut self, ui: &App) {
        self.selected = 0;
        // `scheduleOnlineSearch` runs *synchronously* for an emptied query, so
        // the results go at once rather than 250ms later and a list is never
        // left under a field that has no keyword in it.
        if self.mode == "search-online" && keyword_of(ui).is_empty() {
            self.reset_search(ui);
        }
        self.build(ui);
    }

    /// `scheduleOnlineSearch`'s emptied branch: the results, the spinner and the
    /// error all go, and every request in flight becomes stale.
    fn reset_search(&mut self, ui: &App) {
        self.token += 1;
        self.results.clear();
        self.searching = false;
        self.error = false;
        let state = ui.global::<CommandPaletteState>();
        state.set_online_searching(false);
        state.set_online_error(false);
    }

    /// `runOnlineSearch(source, keyword)`, started by the view's 250ms debounce.
    fn run_search(&mut self, ui: &App) {
        if self.mode != "search-online" {
            return;
        }
        let keyword = keyword_of(ui);
        if keyword.is_empty() {
            self.reset_search(ui);
            return;
        }
        let source = self.source;
        self.token += 1;
        let token = self.token;
        self.searching = true;
        self.error = false;
        let state = ui.global::<CommandPaletteState>();
        state.set_online_searching(true);
        state.set_online_error(false);

        let weak = ui.as_weak();
        crate::runtime::spawn(async move {
            let search = match source {
                "curseforge" => search_curseforge(&keyword).await,
                _ => search_modrinth(&keyword).await,
            };
            let (results, failed) = match search {
                Ok(results) => (results, false),
                Err(error) => {
                    log::error!("command palette search failed: {error}");
                    (Vec::new(), true)
                }
            };
            // Read before `results` moves: each icon belongs to a row, and the
            // row is identified by its position in the results.
            let wanted: Vec<(usize, String)> = results
                .iter()
                .enumerate()
                .map(|(index, result)| (index, result.icon_url.clone()))
                .filter(|(_, url)| !url.is_empty())
                .collect();
            let list_weak = weak.clone();
            let _ = list_weak.upgrade_in_event_loop(move |ui| {
                controller()
                    .borrow_mut()
                    .apply_online_results(&ui, token, results, failed);
            });
            spawn_icon_fetch(weak, token, wanted);
        });
    }

    /// Shows a landed search, unless the user has typed past it — the Vue's
    /// `if (token !== onlineSearchToken) return`, before the answer becomes the
    /// list.
    fn apply_online_results(
        &mut self,
        ui: &App,
        token: u64,
        results: Vec<RemoteResult>,
        failed: bool,
    ) {
        if self.token != token {
            return;
        }
        self.results = results.iter().map(|result| result.to_item(None)).collect();
        self.searching = false;
        self.error = failed;
        // `watch(onlineItems)`: the selection goes back to the first row.
        self.selected = 0;
        let ui_state = ui.global::<CommandPaletteState>();
        ui_state.set_online_searching(false);
        ui_state.set_online_error(failed);
        self.build(ui);
    }
}

/// The trimmed query — the keyword as a search takes it.
///
/// Read off the global rather than kept, because the search field's two-way
/// binding writes it: a second copy would be a second answer to "what has been
/// typed".
pub(crate) fn keyword_of(ui: &App) -> String {
    ui.global::<CommandPaletteState>()
        .get_query()
        .trim()
        .to_string()
}

/// `toInstanceItem`.
pub(crate) fn instance_item(instance: &Instance, action_kind: &'static str) -> PendingItem {
    let runtime = &instance.config.runtime;
    let loader = runtime.mod_loader_type.as_ref().map(ToString::to_string);
    PendingItem {
        key: format!("instance-{}", instance.id),
        title: instance.config.name.clone(),
        icon: "minecraft".to_string(),
        // `${loader} · ${minecraft}` when there is a loader, the version alone
        // when there is not.
        subtitle: match &loader {
            Some(loader) => format!("{loader} · {}", runtime.minecraft),
            None => runtime.minecraft.clone(),
        },
        action_kind,
        action: Action::SelectInstance {
            id: instance.id.clone(),
            // The "launch instance" mode launches what it selects; the root mode's
            // instance list only switches, which is the Vue's `shouldLaunch`.
            launch: action_kind == "launch",
        },
        ..Default::default()
    }
}

pub(crate) fn command_item(command: &Command, first: bool) -> PendingItem {
    PendingItem {
        key: command.key.to_string(),
        title: command.title.to_string(),
        title_kind: command.title_kind.to_string(),
        icon: command.icon.to_string(),
        // A site mark is drawn from its own SVG and takes its ink from a
        // different variable; a glyph comes from the icon set.
        icon_is_brand: command.icon == "modrinth" || command.icon == "curseforge",
        has_children: command.has_children,
        action_kind: "open",
        section_kind: if first { "commands" } else { "" },
        action: command.action.clone(),
        ..Default::default()
    }
}

/// `mod_loader_type ?? "vanilla"`, as the instance filter compares it.
pub(crate) fn loader_name(instance: &Instance) -> String {
    instance
        .config
        .runtime
        .mod_loader_type
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "vanilla".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_heights_follow_the_stylesheet() {
        let plain = PendingItem::default();
        assert_eq!(plain.row_height(), ROW_HEIGHT);
        assert_eq!(plain.label_height(), 0.0);

        let tagged = PendingItem {
            loaders: vec!["fabric".to_string()],
            ..Default::default()
        };
        assert_eq!(tagged.row_height(), ROW_HEIGHT_TAGGED);

        let headed = PendingItem {
            section_kind: "commands",
            ..Default::default()
        };
        assert_eq!(headed.label_height(), LABEL_HEIGHT);
    }

    #[test]
    fn curseforge_classes_follow_the_map() {
        assert_eq!(curseforge_type(Some(6)), "mod");
        assert_eq!(curseforge_type(Some(12)), "resourcepack");
        assert_eq!(curseforge_type(Some(4471)), "modpack");
        // A class the Vue's map has no entry for.
        assert_eq!(curseforge_type(Some(6552)), "");
        assert_eq!(curseforge_type(None), "");
    }

    #[test]
    fn loader_slugs_are_the_only_tags() {
        let categories = serde_json::json!(["fabric", "game-mechanic", "forge", "nope"]);
        assert_eq!(strings(Some(&categories)), vec!["fabric", "forge"]);
    }
}
