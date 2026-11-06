// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The command palette: three modes over one list. The list is built here because
//! Slint can neither build nor index a model, and rows carry label *keys* rather
//! than translated text so a language change re-resolves them. Searches run on the
//! tokio runtime and report back through `upgrade_in_event_loop`.

use std::cell::RefCell;
use std::rc::Rc;

use instance::{Instance, SortBy};
use serde_json::Value;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::slint_backend::{App, CommandPaletteState, Dialogs, GameState, Navigation, PaletteItem};
use crate::ui::overlays::content::{self, PendingImage};

mod search;
mod wiring;

pub(crate) use search::*;
pub(crate) use wiring::*;

thread_local! {
    /// An `upgrade_in_event_loop` closure has to be `Send`, and an `Rc` — which
    /// is what a Slint model is — cannot cross into one. Both the callbacks and
    /// the event-loop closures run on the UI thread and reach it through
    /// [`controller`]; async tasks carry only owned values and a `Weak<App>`.
    static CONTROLLER: Rc<RefCell<PaletteController>> =
        Rc::new(RefCell::new(PaletteController::new()));
}

/// The controller, for use on the UI thread.
pub(crate) fn controller() -> Rc<RefCell<PaletteController>> {
    CONTROLLER.with(Rc::clone)
}

/// `.section-label`'s box plus 4px, so the heading does not sit flush on the row
/// under it.
pub(crate) const LABEL_HEIGHT: f32 = 23.0;
/// `.palette-item` with no loader tags: `padding: 8px 10px` around the 24px icon
/// box, which is taller than the 13px title line.
pub(crate) const ROW_HEIGHT: f32 = 40.0;
/// The same row with the tag line, whose `.item-main` is 30px.
pub(crate) const ROW_HEIGHT_TAGGED: f32 = 46.0;

pub(crate) const SEARCH_LIMIT: usize = 20;

/// How long one project icon may take before the row keeps the site's mark.
///
/// The shared client has no timeout of its own, so a CDN that accepts the
/// connection and then says nothing would hold the icon pass open indefinitely —
/// and with it the next search's.
pub(crate) const ICON_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// One of the root mode's five commands.
///
/// `title` is the source string the *filter* matches against; the row shows
/// `CommandText.title(title_kind, title)`, the translated form.
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

/// What a row does when it is chosen. Resolved when the list is built rather
/// than from the key on the way out, so a callback carrying only an index can
/// find the thing behind it.
#[derive(Clone, Default)]
pub(crate) enum Action {
    #[default]
    None,
    /// `source` is only used for `search-online`.
    EnterMode {
        kind: &'static str,
        source: &'static str,
    },
    CreateInstance,
    AddAccount,
    /// Selects the instance, and opens the launch page when `launch` is set.
    SelectInstance {
        id: String,
        launch: bool,
    },
    /// The game page, then the project's detail panel.
    OpenProject {
        platform: &'static str,
        id: String,
    },
}

/// A row as plain data, before it becomes a `PaletteItem`.
///
/// A `PaletteItem` holds a `ModelRc` and an `Image`, neither of which belongs in
/// an `upgrade_in_event_loop` closure, so results cross as plain data and are
/// finished here on the UI thread.
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
    icon_image: Option<PendingImage>,
    has_children: bool,
    action_kind: &'static str,
    section_kind: &'static str,
    action: Action,
    /// The row's offset from the top of the list, heading included. Only read for
    /// the selected row's reveal; the rows are placed by the list's own layout.
    row_y: f32,
}

impl PendingItem {
    fn label_height(&self) -> f32 {
        if self.section_kind.is_empty() {
            0.0
        } else {
            LABEL_HEIGHT
        }
    }

    fn row_height(&self) -> f32 {
        if self.loaders.is_empty() {
            ROW_HEIGHT
        } else {
            ROW_HEIGHT_TAGGED
        }
    }

    /// The Slint row. Runs on the UI thread, the only place a model may be made.
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
            // The image is drawn whenever the search returned a URL, whether or
            // not the bitmap arrived; the empty image is its broken state.
            icon_image: self.icon(),
            has_icon_image: !self.icon_url.is_empty(),
            has_children: self.has_children,
            action_kind: SharedString::from(self.action_kind),
            section_kind: SharedString::from(self.section_kind),
            label_height: self.label_height(),
            row_height: self.row_height(),
        }
    }

    /// The icon just decoded, or the one an earlier list already decoded. A
    /// project without an icon gets the same unknown world a content card would.
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
            // The site's own mark is used as the project icon when the search
            // returned none.
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
    instances: Vec<Instance>,
    items: Vec<PendingItem>,
    selected: usize,
    /// A reveal on every pointer move would drag the list along with the pointer,
    /// so the sequence the view watches is bumped only when this key changes.
    selected_key: String,
    selected_seq: u64,
    items_seq: u64,
    results: Vec<PendingItem>,
    searching: bool,
    error: bool,
    /// Bumped on every search, so an answer for a request the user has typed past
    /// is dropped.
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

    /// Everything is reset on the way in. The instance list is read on the
    /// runtime and the reset happens in the event loop behind it, so opening the
    /// palette does not wait on a read of every `instance.toml`.
    fn open(ui: &App) {
        let weak = ui.as_weak();
        crate::support::runtime::spawn(async move {
            // Read from the crate rather than borrowed from the game view's rows,
            // which are the grouped, filtered model and not the instance list the
            // palette filters. `filtered` sorts by last played itself.
            let instances = instance::list_instances(SortBy::Playtime)
                .await
                .unwrap_or_default();
            crate::ui::services::report::report(&weak, move |ui| {
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
        self.rebuild(ui);
    }

    fn close(&self, ui: &App) {
        ui.global::<CommandPaletteState>().set_visible(false);
    }

    /// The `Ctrl`/`⌘` + `/` shortcut.
    fn toggle(&mut self, ui: &App) {
        if ui.global::<CommandPaletteState>().get_visible() {
            self.close(ui);
        } else {
            Self::open(ui);
        }
    }

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
        // The view reads these for the placeholder, the breadcrumb, the footer
        // hint and what Escape and Backspace do, so they must be published.
        state.set_mode(SharedString::from(self.mode));
        state.set_source(SharedString::from(self.source));
        self.rebuild(ui);
    }

    fn back_to_root(&mut self, ui: &App) {
        self.enter_mode(ui, "root", self.source);
    }

    fn rebuild(&mut self, ui: &App) {
        self.publish(ui, true)
    }

    /// Puts the rows on screen.
    ///
    /// `reset_scroll` is set when the query, mode or results change, but not when
    /// only icons land: re-running it there would throw the reader back to the
    /// top every time an image arrived.
    fn publish(&mut self, ui: &App, reset_scroll: bool) {
        let state = ui.global::<CommandPaletteState>();
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
                    // The heading belongs to the first row alone.
                    item.section_kind = if index == 0 { "results" } else { "" };
                    item
                })
                .collect(),
            _ => {
                // The commands whose title contains the query, or — when none of
                // them does — the instances. It matches the *source* string; the
                // row still shows the translation, resolved in the view.
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

    fn attach_icons(&mut self, ui: &App, icons: Vec<(usize, Option<PendingImage>)>) {
        for (index, icon) in icons {
            if let Some(item) = self.results.get_mut(index) {
                item.icon_image = icon;
            }
        }
        self.publish(ui, false);
    }

    /// The instances whose name, Minecraft version or loader contains the query,
    /// most recently played first. A never-played instance has no time, so it
    /// sorts last.
    fn filtered(&self, query: &str) -> Vec<&Instance> {
        let mut instances: Vec<&Instance> = self
            .instances
            .iter()
            .filter(|instance| {
                query.is_empty()
                    || [
                        instance.config.name.as_str(),
                        instance.config.runtime.minecraft.as_str(),
                        loader_name(instance).as_str(),
                    ]
                    .iter()
                    .any(|field| field.to_lowercase().contains(query))
            })
            .collect();
        instances.sort_by_key(|instance| std::cmp::Reverse(instance.last_played));
        instances
    }

    fn move_selection(&mut self, ui: &App, delta: i32) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let next = (self.selected as i32 + delta).rem_euclid(count as i32) as usize;
        self.select(ui, next);
    }

    /// The row the pointer is over, which is *not* the selection. The keyboard's
    /// selection stays where it was put and the pointer only says where it is.
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

    /// A click moves the selection to the row, or opens it if already selected.
    /// Enter opens the selection outright, so the pointer and the keyboard reach
    /// the same row in the same two steps.
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
                // The instance is selected *before* the palette closes, so the
                // background has already been told by the time the palette is
                // gone.
                ui.global::<GameState>()
                    .invoke_select_instance(id.clone().into());
                self.close(ui);
                if launch {
                    // The launch button's own entry point, so the palette cannot
                    // start a launch that skips the waiting and the cancel button.
                    crate::ui::views::launch::request(ui, id);
                }
            }
            Action::OpenProject { platform, id } => {
                self.close(ui);
                // The detail panels are mounted by the game view, so the page has
                // to be there before the id is set.
                if ui.global::<Navigation>().get_current_page() != "game" {
                    ui.global::<Navigation>().invoke_navigate("game".into());
                }
                content::open_project_detail(ui, platform, &id);
            }
        }
    }

    fn query_changed(&mut self, ui: &App) {
        self.selected = 0;
        // An emptied query resets the results at once rather than 250ms later.
        if self.mode == "search-online" && keyword_of(ui).is_empty() {
            self.reset_search(ui);
        }
        self.rebuild(ui);
    }

    /// Clears the results, the spinner and the error, and makes every request in
    /// flight stale.
    fn reset_search(&mut self, ui: &App) {
        self.token += 1;
        self.results.clear();
        self.searching = false;
        self.error = false;
        let state = ui.global::<CommandPaletteState>();
        state.set_online_searching(false);
        state.set_online_error(false);
    }

    /// Runs the online search, started by the view's 250ms debounce.
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
        crate::support::runtime::spawn(async move {
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
            crate::ui::services::report::report(&list_weak, move |ui| {
                controller()
                    .borrow_mut()
                    .apply_online_results(&ui, token, results, failed);
            });
            spawn_icon_fetch(weak, token, wanted);
        });
    }

    /// Shows a landed search, unless the user has typed past it, before the
    /// answer becomes the list.
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
        // The results changed, so the selection goes back to the first row.
        self.selected = 0;
        let ui_state = ui.global::<CommandPaletteState>();
        ui_state.set_online_searching(false);
        ui_state.set_online_error(failed);
        self.rebuild(ui);
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

pub(crate) fn instance_item(instance: &Instance, action_kind: &'static str) -> PendingItem {
    let runtime = &instance.config.runtime;
    let loader = runtime.mod_loader_type.as_ref().map(ToString::to_string);
    PendingItem {
        key: format!("instance-{}", instance.id),
        title: instance.config.name.clone(),
        icon: "minecraft".to_string(),
        subtitle: match &loader {
            Some(loader) => format!("{loader} · {}", runtime.minecraft),
            None => runtime.minecraft.clone(),
        },
        action_kind,
        action: Action::SelectInstance {
            id: instance.id.clone(),
            // The "launch instance" mode launches what it selects; the root mode's
            // instance list only switches.
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

/// The mod loader type, or "vanilla" when there is none, as the instance filter
/// compares it.
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
        // A class the map has no entry for.
        assert_eq!(curseforge_type(Some(6552)), "");
        assert_eq!(curseforge_type(None), "");
    }

    #[test]
    fn loader_slugs_are_the_only_tags() {
        let categories = serde_json::json!(["fabric", "game-mechanic", "forge", "nope"]);
        assert_eq!(strings(Some(&categories)), vec!["fabric", "forge"]);
    }
}
