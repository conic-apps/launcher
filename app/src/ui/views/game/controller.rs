// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's controller: the instance list, its rows and the
//! current selection.

use super::*;

use crate::ui::overlays::instance_settings::load_background;

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
        let saved = crate::usecases::game::load();
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
            expanded: saved.expanded.into_iter().collect(),
            accounts: Vec::new(),
            playtime: HashMap::new(),
            avatars: HashMap::new(),
            backgrounds: RefCell::new(HashMap::new()),
            groups: Vec::new(),
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
        Self::reload_selecting(ui, None);
    }

    /// Re-reads the instance list and makes `select` the current instance once
    /// it lands.
    ///
    /// A freshly created instance has to end up selected, but it is not in the
    /// listing loaded at the time: selecting it straight away would leave the
    /// summary empty until the reload completes, and a plain [`Self::reload`]
    /// would keep the instance that was current before. Carrying the id into the
    /// completion sets the list and its selection in the same apply.
    pub(crate) fn reload_selecting(ui: &App, select: Option<String>) {
        let sort = controller().borrow().sort;
        let weak = ui.as_weak();
        crate::support::runtime::spawn(async move {
            // Fall back to an empty listing rather than leaving the page blank:
            // `instance` warns per skipped directory, so this covers only the
            // failure to read the directory at all.
            let instances = match instance::list_instances(sort).await {
                Ok(instances) => instances,
                Err(error) => {
                    log::error!("Could not list the instances: {error}");
                    Vec::new()
                }
            };
            crate::ui::services::report::report(&weak, move |ui| {
                let controller = controller();
                controller.borrow_mut().set_instances(instances);
                if let Some(id) = select {
                    controller.borrow_mut().current_id = Some(id);
                    controller.borrow().persist();
                }
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
        // The content cache is keyed the same way and read the same often, so it
        // is dropped for the same reason: the instance folder may have been
        // edited outside the app since the listing was read.
        crate::ui::overlays::content::cache::clear();
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

    /// (display name, lower-case key) of an instance's mod loader.
    pub(crate) fn loader(instance: &Instance) -> (String, String) {
        match &instance.config.runtime.mod_loader_type {
            Some(loader) => (loader.to_string(), loader.to_string().to_lowercase()),
            None => ("Vanilla".to_string(), "vanilla".to_string()),
        }
    }

    pub(crate) fn expanded(&self, key: &str) -> bool {
        self.expanded.get(key).copied().unwrap_or(true)
    }

    /// Persists the view state: the current id, the sort, the grouping and the
    /// expanded groups.
    ///
    /// It is written on a sort, a grouping, a group toggle and an instance
    /// switch — not on a scroll or a refresh — since a write is a few kilobytes
    /// of JSON.
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
        crate::usecases::game::save(&state);
    }

    /// Builds the flattened list, assigning each row its y/height (so the view
    /// can position rows absolutely and draw the parallax rail) and returning
    /// the total content height plus the current instance's row bounds.
    pub(crate) fn build_rows(&self) -> (Vec<GameRow>, f32, Option<(f32, f32)>) {
        const GAP_TOP: f32 = 132.0;
        const GAP_BOTTOM: f32 = 100.0;
        const GROUP_HEIGHT: f32 = 42.0;
        const CARD_HEIGHT: f32 = 58.0;

        let instances: Vec<&Instance> = self.instances.iter().collect();
        let favorites: Vec<&Instance> = instances
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
                let members: Vec<&Instance> = instances
                    .iter()
                    .copied()
                    .filter(|instance| Self::loader(instance).1 == key)
                    .collect();
                if !members.is_empty() {
                    groups.push((key, "literal", title, members));
                }
            }
        } else if !instances.is_empty() {
            groups.push(("all", "all", "", instances));
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
                has_background: false,
                background: Image::default(),
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

    /// The instance ids in the order the list displays them, paired with the
    /// group each belongs to. Favorites come first; a starred instance that also
    /// belongs to a loader group is listed once, with its first group.
    fn ordered_ids(&self) -> Vec<(String, String)> {
        let instances: Vec<&Instance> = self.instances.iter().collect();
        let favorites: Vec<&Instance> = instances
            .iter()
            .copied()
            .filter(|instance| instance.is_starred())
            .collect();
        let mut groups: Vec<(&str, Vec<&Instance>)> = Vec::new();
        if !favorites.is_empty() {
            groups.push(("starred", favorites));
        }
        if self.group_mode == "loader" {
            for key in ["quilt", "fabric", "neoforge", "forge", "vanilla"] {
                let members: Vec<&Instance> = instances
                    .iter()
                    .copied()
                    .filter(|instance| Self::loader(instance).1 == key)
                    .collect();
                if !members.is_empty() {
                    groups.push((key, members));
                }
            }
        } else if !instances.is_empty() {
            groups.push(("all", instances));
        }

        let mut seen = HashSet::new();
        let mut ordered = Vec::new();
        for (group, members) in groups {
            for instance in members {
                if seen.insert(instance.id.as_str()) {
                    ordered.push((group.to_string(), instance.id.clone()));
                }
            }
        }
        ordered
    }

    /// Moves the current instance one place in the displayed order. Up (-1) and
    /// Down (1) use this, and so does Alt/Option + a wheel notch. Crossing into
    /// a loader group opens that group in the accordion before the row is
    /// scrolled into view.
    ///
    /// Returns whether the selection moved. The ends do not wrap: at the first
    /// or last instance another gesture in the same direction is a no-op.
    pub(crate) fn move_selection(&mut self, direction: i32) -> bool {
        if direction == 0 {
            return false;
        }
        let ordered = self.ordered_ids();
        if ordered.is_empty() {
            return false;
        }
        let current = self
            .current_id
            .as_deref()
            .and_then(|id| ordered.iter().position(|(_, instance)| instance == id))
            .or_else(|| {
                // The persisted id and the row model can briefly disagree (a
                // listing that landed while the old selection was still being
                // restored). The row marked current is the card the player is
                // looking at, so use it before falling back to an end.
                (0..self.rows_model.row_count())
                    .filter_map(|index| self.rows_model.row_data(index))
                    .find(|row| row.current && row.kind.as_str() == "instance")
                    .and_then(|row| {
                        ordered
                            .iter()
                            .position(|(_, instance)| instance.as_str() == row.key.as_str())
                    })
            });
        let next = match current {
            Some(index) => index as i32 + direction,
            None if direction < 0 => (ordered.len() - 1) as i32,
            None => 0,
        };
        if next < 0 || next >= ordered.len() as i32 {
            return false;
        }
        let (group, id) = ordered[next as usize].clone();
        self.adopt_selection(group, id)
    }

    /// The laid-out y of a group header. The toggle callback uses it to bring
    /// the newly opened group's name under the sort/group bar.
    pub(crate) fn group_header_y(&self, key: &str) -> Option<f32> {
        (0..self.rows_model.row_count())
            .filter_map(|index| self.rows_model.row_data(index))
            .find(|row| row.kind.as_str() == "group" && row.key.as_str() == key)
            .map(|row| row.row_y)
    }

    fn adopt_selection(&mut self, group: String, id: String) -> bool {
        if self.current_id.as_deref() == Some(id.as_str()) {
            return false;
        }
        if self.group_mode == "loader" {
            // The target group opens; every other group closes. This is the
            // same accordion rule the group header and the Alt-wheel/arrow
            // stepper use.
            let keys: Vec<String> = self.groups.iter().map(|group| group.key.clone()).collect();
            for key in keys {
                self.expanded.insert(key.clone(), key == group);
            }
            self.expanded.insert(group, true);
            self.flip = true;
        }
        self.current_id = Some(id);
        self.persist();
        true
    }
    /// The group keys in display order and whether each is open. The row list
    /// already carries this, but the accordion normalisation and the stepper
    /// need a compact list that does not depend on a row's height.
    fn group_ranges(rows: &[GameRow]) -> Vec<GroupRange> {
        rows.iter()
            .filter(|row| row.kind.as_str() == "group")
            .map(|row| GroupRange {
                key: row.key.to_string(),
                expanded: !row.collapsed,
            })
            .collect()
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
            has_background: instance.has_background,
            background: self.instance_background(instance),
        }
    }

    /// The instance's decoded background for its list card, memoised by the
    /// background file's modified time so a relayout does not re-read and
    /// re-decode every instance's image. `load_background` reads the file again
    /// on a miss; a cache hit clones the shared pixel buffer.
    fn instance_background(&self, instance: &Instance) -> Image {
        if !instance.has_background {
            // The picture may have just been removed: drop the cached decode.
            self.backgrounds.borrow_mut().remove(&instance.id);
            return Image::default();
        }
        let path = instance::get_background_path(&instance.id);
        let modified = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok();
        if let Some(cached) = self.backgrounds.borrow().get(&instance.id)
            && cached.modified == modified
        {
            return cached.image.clone();
        }
        let image = load_background(instance);
        self.backgrounds.borrow_mut().insert(
            instance.id.clone(),
            CachedBackground {
                modified,
                image: image.clone(),
            },
        );
        image
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

        // Rows that are gone (deleted or regrouped): removed from the
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
        let show_placeholder = self.instances.is_empty();
        let (mut rows, mut content_height, mut current_row) = self.build_rows();
        self.groups = Self::group_ranges(&rows);
        // Grouping is an accordion: one open page, the others collapsed. A
        // saved state from before this was a rule can carry several open groups;
        // keep the one holding the current instance, so the selected instance
        // stays visible, and close the rest.
        if self.group_mode == "loader" {
            let open: Vec<String> = self
                .groups
                .iter()
                .filter(|group| group.expanded)
                .map(|group| group.key.clone())
                .collect();
            if open.len() > 1 {
                let current_group = rows
                    .iter()
                    .find(|row| row.kind.as_str() == "instance" && row.current)
                    .and_then(|row| row.uid.as_str().split(':').next())
                    .map(str::to_string);
                let keep = current_group
                    .filter(|key| open.contains(key))
                    .unwrap_or_else(|| open[0].clone());
                let keys: Vec<String> = self.groups.iter().map(|group| group.key.clone()).collect();
                for key in keys {
                    self.expanded.insert(key.clone(), key == keep);
                }
                self.flip = true;
                let rebuilt = self.build_rows();
                rows = rebuilt.0;
                content_height = rebuilt.1;
                current_row = rebuilt.2;
                self.groups = Self::group_ranges(&rows);
            }
        }
        // The motion kind has been baked into those rows; reset the flag so the
        // next layout defaults to the FLIP.
        self.flip = true;
        // Drop the decodes of instances that no longer exist, so their images
        // do not linger for the rest of the session.
        {
            let live: HashSet<&str> = self.instances.iter().map(|it| it.id.as_str()).collect();
            self.backgrounds
                .borrow_mut()
                .retain(|id, _| live.contains(id.as_str()));
        }

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
                instance.config.background_darkness,
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
        // The four counts and the four preview rows, out of the shared content
        // cache. This runs after `apply_current` because the cache refuses to
        // write to the summary unless it is the current instance — a property
        // only that writes.
        apply_content(ui, current_id.as_deref());
        apply_account(&state, current_account.as_ref(), current_avatar);
        apply_background(ui, background_instance.as_ref());
    }

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
        let unit =
            crate::ui::services::app_config::count_unit(self.config.borrow().language.as_deref());
        state.set_count_unit(unit.into());
    }
}

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

/// The summary's four counts and its four preview rows, out of the content
/// cache the overlay draws from.
///
/// `apply` runs on every list change — a sort, a group toggle, an instance
/// switch — so this is where the cache is asked rather than where the instance
/// is selected: it is a lookup while the cache answers, which is what keeps
/// those three from re-reading the disk. It also writes the counts
/// synchronously, so a fresh cache answers immediately instead of on the next
/// parse to land.
fn apply_content(ui: &App, current_id: Option<&str>) {
    let Some(id) = current_id else {
        // No instance: the rows and the counts are emptied rather than left
        // showing the last one.
        crate::ui::overlays::content::cache::counts(ui, None);
        crate::ui::overlays::content::clear_rows();
        return;
    };
    crate::ui::overlays::content::cache::ensure(ui, id);
    crate::ui::overlays::content::cache::counts(ui, Some(id));
    crate::ui::overlays::content::refresh_preview_icons(ui, id);
}

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

fn apply_background(ui: &App, instance: Option<&(String, bool, bool, u8)>) {
    crate::ui::components::background::controller::set_instance(
        ui,
        instance.map(|(id, use_as_launcher, has_background, darkness)| {
            (id.as_str(), *use_as_launcher, *has_background, *darkness)
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: &str, key: &str, y: f32, height: f32, collapsed: bool) -> GameRow {
        GameRow {
            kind: kind.into(),
            key: key.into(),
            row_y: y,
            height,
            collapsed,
            ..Default::default()
        }
    }

    #[test]
    fn group_ranges_report_the_groups_and_whether_they_are_open() {
        let rows = vec![
            row("group", "forge", 132.0, 42.0, true),
            row("instance", "a", 174.0, 58.0, false),
            row("group", "vanilla", 174.0, 42.0, false),
            row("instance", "b", 216.0, 58.0, false),
        ];
        let ranges = GameController::group_ranges(&rows);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].key, "forge");
        assert!(!ranges[0].expanded);
        assert_eq!(ranges[1].key, "vanilla");
        assert!(ranges[1].expanded);
    }
}
