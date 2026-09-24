// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view "script": drives the Rust domain crates and pushes the results
//! into the `GameState` global (src/views/GameView.vue + src/store/instance.ts,
//! src/store/content.ts and the game composables).

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use chrono::{Datelike, Local, TimeZone};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::slint_backend::{AccountItem, App, GameRow, GameState, Navigation};
use slint_account::Account;
use slint_instance::{Instance, ModLoaderType, SortBy};

/// A ready-to-display relative time (`GameTime.last-played`).
struct RelativeTime {
    kind: &'static str,
    hours: i32,
    month: i32,
    day: i32,
    year: i32,
}

struct GameController {
    config: Rc<RefCell<slint_config::Config>>,
    instances: Vec<Instance>,
    current_id: Option<String>,
    sort: SortBy,
    group_mode: &'static str,
    search: String,
    expanded: HashMap<String, bool>,
    accounts: Vec<Account>,
    playtime: HashMap<String, u64>,
    content: HashMap<String, slint_content::ContentCounts>,
    /// The instance list. It is kept across applies and reconciled in place (see
    /// `sync_rows`), so the view's row items survive a relayout and can animate
    /// along the rail to their new slot instead of being recreated in place.
    rows_model: Rc<VecModel<GameRow>>,
    /// Reveals the rows that `sync_rows` added, one frame after they appeared.
    reveal_timer: Timer,
    /// Whether the list has been handed to the view at least once. The rows of
    /// that first layout are the ones the view's intro slides in, so they start
    /// revealed and only rows added later use the appear animation.
    synced: bool,
    /// How the rows about to be laid out should animate to their new slots. A
    /// group collapse glides 300ms (`easeOutCubic`); everything else is the 400ms
    /// gsap FLIP the Vue original runs for reordered rows. Reset by `apply`.
    flip: bool,
}

impl GameController {
    fn new(config: Rc<RefCell<slint_config::Config>>) -> Self {
        let rows_model = Rc::new(VecModel::<GameRow>::default());
        let reveal_timer = Timer::default();
        {
            // Deferred by a frame: a row that is created with `appear: true`
            // already set is simply placed, so the flag has to flip afterwards for
            // the view to animate it in (see `GameRow::appear`).
            let rows = Rc::clone(&rows_model);
            reveal_timer.start(
                TimerMode::SingleShot,
                Duration::from_millis(50),
                move || {
                    reveal_rows(&rows);
                },
            );
        }
        Self {
            config,
            instances: Vec::new(),
            current_id: None,
            sort: SortBy::Playtime,
            group_mode: "none",
            search: String::new(),
            expanded: HashMap::new(),
            accounts: Vec::new(),
            playtime: HashMap::new(),
            content: HashMap::new(),
            rows_model,
            reveal_timer,
            synced: false,
            flip: true,
        }
    }

    fn reload(&mut self) {
        self.instances = slint_instance::list_instances(self.sort).unwrap_or_default();
        // Re-scan the per-instance caches so a refresh picks up external changes.
        self.playtime.clear();
        self.content.clear();
        // Keep the selection valid, falling back to the first instance.
        if !self
            .current_id
            .as_ref()
            .is_some_and(|id| self.instances.iter().any(|instance| &instance.id == id))
        {
            self.current_id = self.instances.first().map(|instance| instance.id.clone());
        }
        self.reload_accounts();
    }

    fn reload_accounts(&mut self) {
        let accounts = slint_account::list_accounts();
        self.accounts.clear();
        self.accounts
            .extend(accounts.microsoft.into_iter().map(Account::Microsoft));
        self.accounts
            .extend(accounts.yggdrasil.into_iter().map(Account::Yggdrasil));
        self.accounts
            .extend(accounts.offline.into_iter().map(Account::Offline));
    }

    fn current(&self) -> Option<&Instance> {
        let id = self.current_id.as_ref()?;
        self.instances.iter().find(|instance| &instance.id == id)
    }

    fn playtime(&mut self, id: &str) -> u64 {
        if let Some(value) = self.playtime.get(id) {
            return *value;
        }
        let value = slint_instance::calculate_playtime(id).unwrap_or_default();
        self.playtime.insert(id.to_string(), value);
        value
    }

    fn content(&mut self, id: &str) -> slint_content::ContentCounts {
        if let Some(value) = self.content.get(id) {
            return *value;
        }
        let value = slint_content::content_counts(id);
        self.content.insert(id.to_string(), value);
        value
    }

    /// (display name, lower-case key) of an instance's mod loader.
    fn loader(instance: &Instance) -> (String, String) {
        match &instance.config.runtime.mod_loader_type {
            Some(loader) => (loader.to_string(), loader.to_string().to_lowercase()),
            None => ("Vanilla".to_string(), "vanilla".to_string()),
        }
    }

    fn filtered(&self) -> Vec<&Instance> {
        let query = self.search.trim().to_lowercase();
        if query.is_empty() {
            return self.instances.iter().collect();
        }
        self.instances
            .iter()
            .filter(|instance| {
                let runtime = &instance.config.runtime;
                let loader = runtime
                    .mod_loader_type
                    .as_ref()
                    .map(ModLoaderType::to_string)
                    .unwrap_or_else(|| "vanilla".to_string());
                [
                    instance.config.name.clone(),
                    runtime.minecraft.clone(),
                    loader,
                ]
                .iter()
                .any(|field| field.to_lowercase().contains(&query))
            })
            .collect()
    }

    fn expanded(&self, key: &str) -> bool {
        self.expanded.get(key).copied().unwrap_or(true)
    }

    /// Builds the flattened list, assigning each row its y/height (so the view
    /// can position rows absolutely and draw the parallax rail) and returning
    /// the total content height plus the current instance's row bounds.
    fn build_rows(&self) -> (Vec<GameRow>, f32, Option<(f32, f32)>) {
        const GAP_TOP: f32 = 132.0;
        const GAP_BOTTOM: f32 = 100.0;
        const GROUP_HEIGHT: f32 = 42.0;
        const CARD_HEIGHT: f32 = 58.0;

        let filtered = self.filtered();
        let favorites: Vec<&Instance> = filtered
            .iter()
            .copied()
            .filter(|instance| instance.is_starred())
            .collect();

        // (key, title-kind, title-literal, members)
        let mut groups: Vec<(&str, &str, &str, Vec<&Instance>)> = Vec::new();
        if !favorites.is_empty() {
            groups.push(("starred", "favorites", "", favorites));
        }
        if self.group_mode == "loader" {
            for (key, title) in [
                ("quilt", "Quilt"),
                ("fabric", "Fabric"),
                ("neoforge", "Neoforge"),
                ("forge", "Forge"),
                ("vanilla", "Vanilla"),
            ] {
                let members: Vec<&Instance> = filtered
                    .iter()
                    .copied()
                    .filter(|instance| Self::loader(instance).1 == key)
                    .collect();
                if !members.is_empty() {
                    groups.push((key, "literal", title, members));
                }
            }
        } else if !filtered.is_empty() {
            groups.push(("all", "all", "", filtered));
        }

        let mut rows: Vec<GameRow> = Vec::new();
        let mut row_y = GAP_TOP;
        let mut current_row = None;

        for (key, title_kind, title_literal, members) in groups {
            let collapsed = !self.expanded(key);
            rows.push(GameRow {
                kind: "group".into(),
                key: key.into(),
                uid: format!("group-{key}").into(),
                row_y,
                height: GROUP_HEIGHT,
                name: SharedString::default(),
                title_kind: title_kind.into(),
                title_literal: title_literal.into(),
                count: members.len() as i32,
                loader: SharedString::default(),
                loader_key: SharedString::default(),
                last_played_kind: SharedString::default(),
                last_played_hours: 0,
                last_played_month: 0,
                last_played_day: 0,
                last_played_year: 0,
                starred: false,
                current: false,
                collapsed,
                opacity: 1.0,
                appear: true,
                flip: self.flip,
            });
            row_y += GROUP_HEIGHT;

            // A collapsed group keeps its cards in the model, in the positions
            // they would have when open, with `opacity: 0`. The original shrinks
            // the group's box while its cards fade where they stand (they are in
            // normal flow there), so the group has to fade as a whole — the gap is
            // closed by the rows below gliding up, never by the cards moving.
            let mut member_y = row_y;
            for instance in members {
                let opacity = if collapsed { 0.0 } else { 1.0 };
                rows.push(self.instance_row(instance, key, member_y, CARD_HEIGHT, opacity));
                member_y += CARD_HEIGHT;
                if collapsed {
                    continue;
                }
                // The current instance is centred on the *first* row that shows it,
                // which is the starred copy when it has one (the Vue `scrollTo`
                // matches the first `data-id`).
                if current_row.is_none() && self.current_id.as_deref() == Some(instance.id.as_str())
                {
                    current_row = Some((row_y, CARD_HEIGHT));
                }
                row_y += CARD_HEIGHT;
            }
        }

        (rows, row_y + GAP_BOTTOM, current_row)
    }

    fn instance_row(
        &self,
        instance: &Instance,
        group: &str,
        row_y: f32,
        height: f32,
        opacity: f32,
    ) -> GameRow {
        let (loader, loader_key) = Self::loader(instance);
        let relative = relative_time(instance.last_played);
        GameRow {
            kind: "instance".into(),
            key: instance.id.clone().into(),
            uid: format!("{group}:{}", instance.id).into(),
            row_y,
            height,
            name: instance.config.name.clone().into(),
            title_kind: SharedString::default(),
            title_literal: SharedString::default(),
            count: 0,
            loader: loader.into(),
            loader_key: loader_key.into(),
            last_played_kind: relative.kind.into(),
            last_played_hours: relative.hours,
            last_played_month: relative.month,
            last_played_day: relative.day,
            last_played_year: relative.year,
            starred: instance.is_starred(),
            current: self.current_id.as_deref() == Some(instance.id.as_str()),
            collapsed: false,
            opacity,
            appear: true,
            flip: self.flip,
        }
    }

    /// Merges the freshly laid-out rows into the model the view is rendering.
    ///
    /// Rows are matched by key rather than by position: reordering (sorting,
    /// grouping, starring) only changes their `row_y`, and the view animates the
    /// rows it already has. Resetting the model instead would destroy and recreate
    /// every row item, which is exactly what would stop them gliding along the
    /// rail.
    fn sync_rows(&mut self, rows: Vec<GameRow>) {
        let model = &self.rows_model;
        let first_layout = !self.synced;
        let order: Vec<SharedString> = rows.iter().map(|row| row.uid.clone()).collect();
        let mut incoming: HashMap<SharedString, GameRow> =
            rows.into_iter().map(|row| (row.uid.clone(), row)).collect();

        // Rows we already render: update in place, keeping their identity (and so
        // whatever animation is currently running on them). What was matched is
        // remembered separately — the map itself has to stay whole here, it is
        // what the append pass below takes the new rows from.
        let mut kept: HashSet<SharedString> = HashSet::new();
        for index in 0..model.row_count() {
            let Some(current) = model.row_data(index) else {
                continue;
            };
            let Some(target) = incoming.get(&current.uid) else {
                continue;
            };
            let mut row = target.clone();
            kept.insert(current.uid.clone());
            // A row that has not been revealed yet must not be revealed early by
            // an unrelated refresh; the timer owns that transition.
            row.appear = current.appear;
            if row != current {
                model.set_row_data(index, row);
            }
        }

        // Rows that are gone (deleted, filtered out, regrouped): removed from the
        // back so the indices below stay valid.
        for index in (0..model.row_count()).rev() {
            let Some(current) = model.row_data(index) else {
                continue;
            };
            if !kept.contains(&current.uid) {
                model.remove(index);
            }
        }

        // Rows that are new to the list are appended hidden and revealed by the
        // timer, which is what animates them in. The very first layout is the
        // exception: those rows are the ones the view's intro slides in, so they
        // are handed over already visible.
        for key in order {
            if kept.contains(&key) {
                continue;
            }
            if let Some(mut row) = incoming.remove(&key) {
                row.appear = first_layout;
                model.push(row);
            }
        }

        self.synced = true;
        self.reveal_timer.restart();
    }

    fn apply(&mut self, ui: &App) {
        let filtered = self.filtered();
        let show_placeholder =
            self.instances.is_empty() || (!self.search.trim().is_empty() && filtered.is_empty());
        let (rows, content_height, current_row) = self.build_rows();
        // The motion kind has been baked into those rows: everything that follows
        // is a FLIP unless a collapse asks for its own timing.
        self.flip = true;

        // Content counts + summary need the current instance; compute them
        // before borrowing the global mutably.
        let content = self
            .current_id
            .clone()
            .map(|id| self.content(&id))
            .filter(|_| self.current().is_some());
        let playtime = self
            .current_id
            .clone()
            .map(|id| self.playtime(&id))
            .unwrap_or_default();
        let current = self.current().cloned();

        let accounts: Vec<AccountItem> = self
            .accounts
            .iter()
            .map(|account| AccountItem {
                key: account.key().into(),
                name: account.profile_name().into(),
                kind: account.kind().into(),
            })
            .collect();
        let current_account = self.config.borrow().current_account.clone();

        self.sync_rows(rows);

        let state = ui.global::<GameState>();
        state.set_list_content_height(content_height);
        match current_row {
            Some((y, height)) => {
                state.set_current_row_y(y);
                state.set_current_row_height(height);
            }
            None => {
                state.set_current_row_y(-1.0);
                state.set_current_row_height(0.0);
            }
        }
        state.set_has_instances(!self.instances.is_empty());
        state.set_show_placeholder(show_placeholder);
        state.set_accounts(ModelRc::new(VecModel::from(accounts)));

        match current {
            Some(instance) => {
                let (loader, _) = Self::loader(&instance);
                let relative = relative_time(instance.last_played);
                state.set_has_current(true);
                state.set_current_name(instance.config.name.clone().into());
                state.set_current_minecraft(instance.config.runtime.minecraft.clone().into());
                state.set_current_has_loader(instance.config.runtime.mod_loader_type.is_some());
                state.set_current_loader(loader.into());
                state.set_current_loader_version(
                    instance
                        .config
                        .runtime
                        .mod_loader_version
                        .clone()
                        .unwrap_or_default()
                        .into(),
                );
                state.set_current_last_played_kind(relative.kind.into());
                state.set_current_last_played_hours(relative.hours);
                state.set_current_last_played_month(relative.month);
                state.set_current_last_played_day(relative.day);
                state.set_current_last_played_year(relative.year);
                state.set_current_starred(instance.is_starred());
                state.set_current_has_playtime(playtime > 0);
                let (kind, value) = format_play_time(playtime);
                state.set_current_playtime_kind(kind.into());
                state.set_current_playtime_value(value.into());
            }
            None => {
                state.set_has_current(false);
                state.set_current_has_loader(false);
                state.set_current_has_playtime(false);
                state.set_current_starred(false);
            }
        }

        match content {
            Some(content) => {
                state.set_content_saves(content.saves as i32);
                state.set_content_mods(content.mods as i32);
                state.set_content_resourcepacks(content.resourcepacks as i32);
                state.set_content_screenshots(content.screenshots as i32);
            }
            None => {
                state.set_content_saves(0);
                state.set_content_mods(0);
                state.set_content_resourcepacks(0);
                state.set_content_screenshots(0);
            }
        }

        // The count unit follows the active locale (`CONIC_LOCALE` included).
        let unit = crate::config_bridge::count_unit(self.config.borrow().language.as_deref());
        state.set_count_unit(unit.into());

        state.set_has_account(current_account.is_some());
        match current_account {
            Some(account) => {
                state.set_current_account_key(account.key().into());
                state.set_current_account_name(account.profile_name().into());
                state.set_current_account_kind(account.kind().into());
            }
            None => {
                state.set_current_account_key(SharedString::default());
                state.set_current_account_name(SharedString::default());
                state.set_current_account_kind(SharedString::default());
            }
        }
    }
}

/// Reveals the rows added by the last `sync_rows`: `appear` flipping to true is
/// what makes the view animate them in along the rail.
fn reveal_rows(model: &VecModel<GameRow>) {
    for index in 0..model.row_count() {
        let Some(mut row) = model.row_data(index) else {
            continue;
        };
        if !row.appear {
            row.appear = true;
            model.set_row_data(index, row);
        }
    }
}

/// Formats a play time in seconds into a `GameTime.play-time` kind + value.
fn format_play_time(seconds: u64) -> (&'static str, String) {
    if seconds < 60 {
        return ("seconds", seconds.to_string());
    }
    let minutes = seconds as f64 / 60.0;
    if minutes < 60.0 {
        return ("minutes", format_decimal(minutes));
    }
    ("hours", format_decimal(minutes / 60.0))
}

/// Rounds to one decimal and strips a trailing `.0` (mirrors
/// `Number(value.toFixed(1)).toString()`).
fn format_decimal(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// Resolves the relative-time parts of a last-played timestamp (mirrors
/// `formatLastPlayed` in crates/instance/index.ts).
fn relative_time(timestamp: Option<u64>) -> RelativeTime {
    let Some(timestamp) = timestamp else {
        return RelativeTime {
            kind: "never",
            hours: 0,
            month: 0,
            day: 0,
            year: 0,
        };
    };
    // The launch script stores the timestamp in milliseconds (the Vue
    // `formatLastPlayed` passes it straight to `new Date(timestamp)`).
    let Some(date) = Local.timestamp_millis_opt(timestamp as i64).single() else {
        return RelativeTime {
            kind: "never",
            hours: 0,
            month: 0,
            day: 0,
            year: 0,
        };
    };
    let now = Local::now();

    if date.date_naive() == now.date_naive() {
        let hours = (now - date).num_hours().max(0);
        if hours < 1 {
            return RelativeTime {
                kind: "just-now",
                hours: 0,
                month: 0,
                day: 0,
                year: 0,
            };
        }
        return RelativeTime {
            kind: "hours-ago",
            hours: hours as i32,
            month: 0,
            day: 0,
            year: 0,
        };
    }

    let yesterday = now.date_naive() - chrono::Duration::days(1);
    if date.date_naive() == yesterday {
        return RelativeTime {
            kind: "yesterday",
            hours: 0,
            month: 0,
            day: 0,
            year: 0,
        };
    }

    if date.year() == now.year() {
        return RelativeTime {
            kind: "month-day",
            hours: 0,
            month: date.month() as i32,
            day: date.day() as i32,
            year: 0,
        };
    }

    RelativeTime {
        kind: "year-month-day",
        hours: 0,
        month: date.month() as i32,
        day: date.day() as i32,
        year: date.year(),
    }
}

/// Registers every game view callback on the `GameState` global.
pub fn setup(ui: &App, config: Rc<RefCell<slint_config::Config>>) {
    let controller = Rc::new(RefCell::new(GameController::new(config)));
    // The list model is handed to the view once and then kept in sync in place by
    // `sync_rows`, so the rows the view renders are never recreated.
    ui.global::<GameState>()
        .set_rows(ModelRc::new(controller.borrow().rows_model.clone()));
    controller.borrow_mut().reload();
    controller.borrow_mut().apply(ui);

    let state = ui.global::<GameState>();

    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_refresh(move || {
            controller.borrow_mut().reload();
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_select_instance(move |id| {
            let id = id.to_string();
            if controller.borrow().current_id.as_deref() == Some(id.as_str()) {
                return;
            }
            controller.borrow_mut().current_id = Some(id);
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_toggle_group(move |key| {
            let key = key.to_string();
            let mut controller = controller.borrow_mut();
            let expanded = controller.expanded(&key);
            // Collapsing closes the gap on the collapse curve; re-opening is a
            // FLIP, like every other change to the layout (the Vue original
            // animates a collapse by hand and everything else with its FLIP).
            controller.flip = expanded;
            controller.expanded.insert(key, !expanded);
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_set_sort(move |mode| {
            let sort = match mode.as_str() {
                "name" => SortBy::Name,
                "version" => SortBy::Version,
                "lastplay" => SortBy::LastPlayed,
                _ => SortBy::Playtime,
            };
            let mut controller = controller.borrow_mut();
            controller.sort = sort;
            controller.reload();
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_set_group_mode(move |mode| {
            let mut controller = controller.borrow_mut();
            controller.group_mode = if mode.as_str() == "loader" {
                "loader"
            } else {
                "none"
            };
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_search_changed(move |query| {
            controller.borrow_mut().search = query.to_string();
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_select_account(move |key| {
            let key = key.to_string();
            let account = controller
                .borrow()
                .accounts
                .iter()
                .find(|account| account.key() == key)
                .cloned();
            let Some(account) = account else { return };
            let config_rc = controller.borrow().config.clone();
            config_rc.borrow_mut().current_account = Some(account);
            let _ = slint_config::save_config(&config_rc.borrow());
            if let Some(ui) = weak.upgrade() {
                controller.borrow_mut().apply(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_launch(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Navigation>().invoke_navigate("launch".into());
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_repair_and_launch(move || {
            let Some(id) = controller.borrow().current_id.clone() else {
                return;
            };
            if let Err(error) = slint_instance::remove_install_lock(&id) {
                log::error!("failed to remove install lock: {error}");
                return;
            }
            if let Some(ui) = weak.upgrade() {
                // The launch page re-runs the install flow when the lock is gone.
                ui.global::<Navigation>().invoke_navigate("launch".into());
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        state.on_open_instance_folder(move || {
            let Some(id) = controller.borrow().current_id.clone() else {
                return;
            };
            let path = slint_folder::DATA_LOCATION.get_instance_root(&id);
            if let Err(error) = crate::config_bridge::open_external(&path.to_string_lossy()) {
                log::warn!("failed to open instance folder: {error}");
            }
        });
    }
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_toggle_starred(move || {
            let mut controller = controller.borrow_mut();
            let Some(instance) = controller.current().cloned() else {
                return;
            };
            let mut config = instance.config.clone();
            let mut groups = config.group.clone().unwrap_or_default();
            if instance.is_starred() {
                groups.retain(|group| group != "starred");
            } else {
                groups.push("starred".to_string());
            }
            config.group = Some(groups);
            if let Err(error) = slint_instance::update_instance(config, &instance.id) {
                log::error!("failed to update instance: {error}");
                return;
            }
            controller.reload();
            if let Some(ui) = weak.upgrade() {
                controller.apply(&ui);
            }
        });
    }

    // The remaining callbacks open overlays/dialogs that are not migrated yet.
    state.on_open_instance_settings(
        || log::info!(target: "game", "instance settings (not migrated)"),
    );
    state
        .on_open_content(|kind| log::info!(target: "game", "open content '{kind}' (not migrated)"));
    state.on_open_add_account(|| log::info!(target: "game", "add account (not migrated)"));
    state.on_open_connect(|| log::info!(target: "game", "multiplayer connect (not migrated)"));
    state.on_new_instance(|| log::info!(target: "game", "create instance (not migrated)"));
    state.on_open_packs(|| log::info!(target: "game", "install packs (not migrated)"));
    state.on_debug_log(|message| log::info!(target: "probe", "{message}"));
    // The list's smoothing scales its per-tick step by the elapsed time, the same
    // way Lenis does, so the glide does not depend on the display's refresh rate.
    // The clock is monotonic: a wall clock can be stepped (NTP), which would make
    // the step jump or stall.
    state.on_now_ms(|| {
        static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        ORIGIN
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f32()
            * 1000.0
    });
    {
        let weak = ui.as_weak();
        state.on_open_accounts(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Navigation>().invoke_navigate("accounts".into());
            }
        });
    }
}
