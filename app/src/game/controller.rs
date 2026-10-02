// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's controller: the instance list, its rows and the
//! current selection.

use super::*;

impl GameController {
    pub(crate) fn new(config: Rc<RefCell<config::Config>>) -> Self {
        let rows_model = Rc::new(VecModel::<GameRow>::default());
        let reveal_timer = Timer::default();
        {
            // Deferred by 50ms: a row that is created with `appear: true`
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
        // The persisted view state, read back before the first paint so a
        // restart comes back to the list the user left.
        let saved = instance_view::load();
        let sort = match saved.sort_mode() {
            SavedSortMode::Name => SortBy::Name,
            SavedSortMode::Version => SortBy::Version,
            SavedSortMode::LastPlay => SortBy::LastPlayed,
            SavedSortMode::Playtime => SortBy::Playtime,
        };
        let group_mode = match saved.group() {
            GroupMode::Loader => "loader",
            GroupMode::None => "none",
        };
        // The id is not checked against the listing here: the instances have not
        // been read yet. `set_instances` drops it if the instance is gone,
        // falling back to the first instance.
        let current_id = (!saved.current_id.is_empty()).then_some(saved.current_id);
        Self {
            config,
            instances: Vec::new(),
            current_id,
            sort,
            group_mode,
            search: String::new(),
            expanded: saved.expanded.into_iter().collect(),
            accounts: Vec::new(),
            playtime: HashMap::new(),
            content: HashMap::new(),
            avatars: HashMap::new(),
            rows_model,
            reveal_timer,
            synced: false,
            flip: true,
        }
    }

    /// Re-reads the instance list on the runtime and applies it in the event
    /// loop, so the listing — one `instance.toml` read and parse per instance —
    /// never runs on the thread that draws.
    pub(crate) fn reload(ui: &App) {
        let sort = controller().borrow().sort;
        let weak = ui.as_weak();
        crate::runtime::spawn(async move {
            let instances = instance::list_instances(sort).await.unwrap_or_default();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                let controller = controller();
                controller.borrow_mut().set_instances(instances);
                controller.borrow_mut().apply(&ui);
            });
        });
    }

    /// Adopts a freshly listed set of instances and re-reads everything derived
    /// from it. Runs on the UI thread, on the far side of [`GameController::reload`].
    pub(crate) fn set_instances(&mut self, instances: Vec<Instance>) {
        self.instances = instances;
        // Re-scan the per-instance caches so a refresh picks up external changes.
        self.playtime.clear();
        self.content.clear();
        // Keep the selection valid, falling back to the first instance and
        // persisting it, so a deleted instance stops being restored on the next
        // run.
        let kept = self
            .current_id
            .as_ref()
            .is_some_and(|id| self.instances.iter().any(|instance| &instance.id == id));
        if !kept {
            self.current_id = self.instances.first().map(|instance| instance.id.clone());
            self.persist();
        }
        self.reload_accounts();
    }

    pub(crate) fn reload_accounts(&mut self) {
        let accounts = account::list_accounts();
        self.accounts.clear();
        // The set of accounts is what the cache is keyed on.
        self.avatars.clear();
        self.accounts
            .extend(accounts.microsoft.into_iter().map(Account::Microsoft));
        self.accounts
            .extend(accounts.yggdrasil.into_iter().map(Account::Yggdrasil));
        self.accounts
            .extend(accounts.offline.into_iter().map(Account::Offline));
    }

    pub(crate) fn current(&self) -> Option<&Instance> {
        let id = self.current_id.as_ref()?;
        self.instances.iter().find(|instance| &instance.id == id)
    }

    pub(crate) fn playtime(&mut self, id: &str) -> u64 {
        if let Some(value) = self.playtime.get(id) {
            return *value;
        }
        let value = instance::calculate_playtime(id).unwrap_or_default();
        self.playtime.insert(id.to_string(), value);
        value
    }

    pub(crate) fn content(&mut self, id: &str) -> content::ContentCounts {
        if let Some(value) = self.content.get(id) {
            return *value;
        }
        let value = content::content_counts(id);
        self.content.insert(id.to_string(), value);
        value
    }

    /// (display name, lower-case key) of an instance's mod loader.
    pub(crate) fn loader(instance: &Instance) -> (String, String) {
        match &instance.config.runtime.mod_loader_type {
            Some(loader) => (loader.to_string(), loader.to_string().to_lowercase()),
            None => ("Vanilla".to_string(), "vanilla".to_string()),
        }
    }

    pub(crate) fn filtered(&self) -> Vec<&Instance> {
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

    pub(crate) fn expanded(&self, key: &str) -> bool {
        self.expanded.get(key).copied().unwrap_or(true)
    }

    /// Persists the view state: the current id, the sort, the grouping and the
    /// expanded groups.
    ///
    /// It is written on a sort, a grouping, a group toggle and an instance
    /// switch — not on a scroll, a search or a refresh — since a write is a few
    /// kilobytes of JSON.
    pub(crate) fn persist(&self) {
        let state = InstanceViewState {
            current_id: self.current_id.clone().unwrap_or_default(),
            sort: match self.sort {
                SortBy::Name => "name",
                SortBy::Version => "version",
                SortBy::LastPlayed => "lastplay",
                SortBy::Playtime => "playtime",
            }
            .to_string(),
            group_mode: match self.group_mode {
                "loader" => "loader",
                _ => "none",
            }
            .to_string(),
            expanded: self
                .expanded
                .iter()
                .map(|(key, value)| (key.clone(), *value))
                .collect(),
        };
        instance_view::save(&state);
    }

    /// Builds the flattened list, assigning each row its y/height (so the view
    /// can position rows absolutely and draw the parallax rail) and returning
    /// the total content height plus the current instance's row bounds.
    pub(crate) fn build_rows(&self) -> (Vec<GameRow>, f32, Option<(f32, f32)>) {
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
            // they would have when open, with `opacity: 0`. The group fades as a
            // whole while its cards stay put — the gap is closed by the rows
            // below gliding up, never by the cards moving.
            let mut member_y = row_y;
            for instance in members {
                let opacity = if collapsed { 0.0 } else { 1.0 };
                rows.push(self.instance_row(instance, key, member_y, CARD_HEIGHT, opacity));
                member_y += CARD_HEIGHT;
                if collapsed {
                    continue;
                }
                // The current instance is centred on the *first* row that shows
                // it, which is the starred copy when it has one.
                if current_row.is_none() && self.current_id.as_deref() == Some(instance.id.as_str())
                {
                    current_row = Some((row_y, CARD_HEIGHT));
                }
                row_y += CARD_HEIGHT;
            }
        }

        (rows, row_y + GAP_BOTTOM, current_row)
    }

    pub(crate) fn instance_row(
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
    /// Rows are matched by `uid` rather than by position: reordering (sorting,
    /// grouping, starring) only changes their `row_y`, and the view animates the
    /// rows it already has. Resetting the model instead would destroy and recreate
    /// every row item, which is exactly what would stop them gliding along the
    /// rail.
    pub(crate) fn sync_rows(&mut self, rows: Vec<GameRow>) {
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

    pub(crate) fn apply(&mut self, ui: &App) {
        let filtered = self.filtered();
        let show_placeholder =
            self.instances.is_empty() || (!self.search.trim().is_empty() && filtered.is_empty());
        let (rows, content_height, current_row) = self.build_rows();
        // The motion kind has been baked into those rows; reset the flag so the
        // next layout defaults to the FLIP.
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
        // The window background follows the current instance (the summary below
        // consumes `current`, so this is taken first).
        let background_instance = current.as_ref().map(|instance| {
            (
                instance.id.clone(),
                instance.config.use_as_launcher_background,
                instance.has_background,
            )
        });

        // Taken out for the map below, which needs the cache mutably while it
        // borrows the account list immutably, and put back afterwards.
        let mut avatars = std::mem::take(&mut self.avatars);
        let accounts: Vec<AccountItem> = self
            .accounts
            .iter()
            .map(|account| AccountItem {
                key: account.key().into(),
                name: account.get_profile_name().into(),
                kind: account.kind().into(),
                avatar: account_avatar(Some(account), 18, &mut avatars),
            })
            .collect();
        let current_account = self.config.borrow().current_account.clone();
        let current_avatar = account_avatar(current_account.as_ref(), 56, &mut avatars);
        self.avatars = avatars;

        self.sync_rows(rows);

        let state = ui.global::<GameState>();
        self.apply_list(
            &state,
            content_height,
            current_row,
            show_placeholder,
            accounts,
        );
        let current_id = current.as_ref().map(|instance| instance.id.clone());
        apply_current(&state, current.as_ref(), playtime);
        // The preview rows draw the first few icons of each kind as well as
        // their counts; `content` owns the decoding and the caches.
        apply_preview_rows(ui, &state, current_id.as_deref());
        apply_content_counts(&state, content);
        apply_account(&state, current_account.as_ref(), current_avatar);
        apply_background(ui, background_instance.as_ref());
    }

    /// Pushes the list's own view state: the sort, the grouping, the height, the
    /// current row and the account rows.
    fn apply_list(
        &self,
        state: &GameState<'_>,
        content_height: f32,
        current_row: Option<(f32, f32)>,
        show_placeholder: bool,
        accounts: Vec<AccountItem>,
    ) {
        state.set_sort_mode(
            match self.sort {
                SortBy::Name => "name",
                SortBy::Version => "version",
                SortBy::LastPlayed => "lastplay",
                SortBy::Playtime => "playtime",
            }
            .into(),
        );
        state.set_group_mode(self.group_mode.into());
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
        // The count unit follows the active locale (`CONIC_LOCALE` included).
        let unit = crate::config_bridge::count_unit(self.config.borrow().language.as_deref());
        state.set_count_unit(unit.into());
    }
}

/// Pushes the current instance's summary, or clears it when none is selected.
fn apply_current(state: &GameState<'_>, current: Option<&Instance>, playtime: u64) {
    match current {
        Some(instance) => {
            let (loader, _) = GameController::loader(instance);
            let relative = relative_time(instance.last_played);
            state.set_has_current(true);
            state.set_current_id(instance.id.clone().into());
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
            state.set_current_id(SharedString::default());
            state.set_current_has_loader(false);
            state.set_current_has_playtime(false);
            state.set_current_starred(false);
        }
    }
}

/// The current instance's content counts.
fn apply_content_counts(state: &GameState<'_>, content: Option<content::ContentCounts>) {
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
}

/// The preview rows of the current instance, or four empty models without one.
fn apply_preview_rows(ui: &App, state: &GameState<'_>, current_id: Option<&str>) {
    match current_id {
        Some(id) => crate::content::refresh_preview_icons(ui, id),
        None => {
            state.set_preview_saves(slint::ModelRc::default());
            state.set_preview_mods(slint::ModelRc::default());
            state.set_preview_resourcepacks(slint::ModelRc::default());
            state.set_preview_screenshots(slint::ModelRc::default());
        }
    }
}

/// The selected account, or the logged-out footer.
fn apply_account(state: &GameState<'_>, account: Option<&Account>, avatar: Image) {
    state.set_has_account(account.is_some());
    state.set_current_account_avatar(avatar);
    match account {
        Some(account) => {
            state.set_current_account_key(account.key().into());
            state.set_current_account_name(account.get_profile_name().into());
            state.set_current_account_kind(account.kind().into());
        }
        None => {
            state.set_current_account_key(SharedString::default());
            state.set_current_account_name(SharedString::default());
            state.set_current_account_kind(SharedString::default());
        }
    }
}

/// Hands the current instance to the background controller: its own image when
/// it asks to be the launcher's, the global one, or the 3D world.
fn apply_background(ui: &App, instance: Option<&(String, bool, bool)>) {
    crate::background::controller::set_instance(
        ui,
        instance.map(|(id, use_as_launcher, has_background)| {
            (id.as_str(), *use_as_launcher, *has_background)
        }),
    );
}
