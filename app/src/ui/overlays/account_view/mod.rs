// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The account view's script: builds the account-details overlay's state and
//! answers its callbacks.
//!
//! The overlay is opened from the footer's avatar (`game`), which only calls
//! [`open()`]; everything the panel shows — the head, the auth details, the launch
//! contribution graph, the recent activity and the skin / cape preview — is
//! assembled here from the account files and `statistics.json`.
//!
//! The account files are read synchronously (`account::list_accounts` is a few
//! kilobytes), so the header can be drawn in the same frame the overlay opens.
//! The two slower reads — the statistics and a Yggdrasil server's name — run on
//! the runtime and land later.

mod skin;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use account::{Account, offline::OfflineAccount};
use statistics::{StatisticsEntry, StatisticsProfile};

use crate::slint_backend::{
    AccountActivity, AccountDay, AccountMonthLabel, AccountViewState, App, CapeItem,
    DeleteAccountState, Dialogs, GameState,
};
use crate::ui::services::report::report;

// The account the overlay is showing. Kept here rather than in the global: the
// global carries the *rendered* fields, and an `Account` is not a Slint type.
thread_local! {
    static VIEWED: RefCell<Option<Account>> = const { RefCell::new(None) };
    /// The launcher's config, for the "set as default" and delete paths. Set by
    /// [`setup`]; accessed on the UI thread only.
    static CONFIG: RefCell<Option<Rc<RefCell<config::Config>>>> = const { RefCell::new(None) };
}

fn config() -> Option<Rc<RefCell<config::Config>>> {
    CONFIG.with(|cell| cell.borrow().clone())
}

pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    CONFIG.with(|cell| *cell.borrow_mut() = Some(config));

    let state = ui.global::<AccountViewState>();
    {
        let weak = ui.as_weak();
        state.on_close(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<AccountViewState>().set_visible(false);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_select_account(move |key| {
            if let Some(ui) = weak.upgrade() {
                select(&ui, key.as_str());
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_add_account(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Dialogs>().set_account_add_visible(true);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_delete_account(move || {
            if let Some(ui) = weak.upgrade() {
                open_delete_dialog(&ui);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_skin_action(move |kind| {
            if let Some(ui) = weak.upgrade() {
                skin_action(&ui, kind.as_str());
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_select_cape(move |index| {
            if let Some(ui) = weak.upgrade() {
                select_cape(&ui, index);
            }
        });
    }

    let delete = ui.global::<DeleteAccountState>();
    {
        let weak = ui.as_weak();
        delete.on_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Dialogs>()
                    .set_confirm_delete_account_visible(false);
            }
        });
    }
    {
        let weak = ui.as_weak();
        delete.on_confirm(move || {
            if let Some(ui) = weak.upgrade() {
                confirm_delete(&ui);
            }
        });
    }
}

/// Every stored account, flattened into the order the UI lists them.
fn all_accounts() -> Vec<Account> {
    let accounts = account::list_accounts();
    accounts
        .microsoft
        .into_iter()
        .map(Account::Microsoft)
        .chain(accounts.offline.into_iter().map(Account::Offline))
        .chain(accounts.yggdrasil.into_iter().map(Account::Yggdrasil))
        .collect()
}

/// Opens the overlay on the launcher's current account (or the first stored
/// one).
pub fn open(ui: &App) {
    let accounts = all_accounts();
    let current = config().and_then(|config| config.borrow().current_account.clone());
    let viewed = current
        .filter(|account| accounts.iter().any(|stored| stored.key() == account.key()))
        .or_else(|| accounts.first().cloned());
    VIEWED.with(|cell| *cell.borrow_mut() = viewed);
    ui.global::<AccountViewState>().set_visible(true);
    refresh(ui);
}

/// Called after the add-account dialog finishes: if the overlay is up, keep it
/// consistent with what was just written.
pub fn on_accounts_changed(ui: &App) {
    if !ui.global::<AccountViewState>().get_visible() {
        return;
    }
    let accounts = all_accounts();
    let viewed = VIEWED.with(|cell| {
        let current = cell.borrow().clone();
        current
            .filter(|account| accounts.iter().any(|stored| stored.key() == account.key()))
            .or_else(|| accounts.first().cloned())
    });
    VIEWED.with(|cell| *cell.borrow_mut() = viewed);
    refresh(ui);
}

fn select(ui: &App, key: &str) {
    let account = all_accounts()
        .into_iter()
        .find(|account| account.key() == key);
    let Some(account) = account else {
        return;
    };
    // Choosing here is the same act as the footer's switcher: it changes the
    // launcher's *current* account, not just what the overlay is looking at, so
    // the footer and the launch page follow. The overlay shows that account too.
    if let Some(config) = config() {
        config.borrow_mut().current_account = Some(account.clone());
        if let Err(error) = config::save_config(&config.borrow()) {
            log::warn!("failed to save the config: {error}");
        }
    }
    VIEWED.with(|cell| *cell.borrow_mut() = Some(account));
    ui.global::<GameState>().invoke_refresh();
    refresh(ui);
}

fn open_delete_dialog(ui: &App) {
    let Some(account) = VIEWED.with(|cell| cell.borrow().clone()) else {
        return;
    };
    let state = ui.global::<DeleteAccountState>();
    state.set_key(account.key().into());
    state.set_name(account.get_profile_name().into());
    state.set_deleting(false);
    ui.global::<Dialogs>()
        .set_confirm_delete_account_visible(true);
}

fn confirm_delete(ui: &App) {
    let Some(account) = VIEWED.with(|cell| cell.borrow().clone()) else {
        return;
    };
    ui.global::<DeleteAccountState>().set_deleting(true);
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let result = match &account {
            Account::Microsoft(account) => {
                account::microsoft::delete_account(account.profile.uuid).await
            }
            Account::Offline(account) => account::offline::delete_account(account.uuid).await,
            Account::Yggdrasil(account) => {
                account::yggdrasil::delete_account(account.clone()).await
            }
        };
        if let Err(error) = result {
            log::error!("failed to delete the account: {error}");
        }
        let key = account.key();
        report(&weak, move |ui| {
            ui.global::<DeleteAccountState>().set_deleting(false);
            ui.global::<Dialogs>()
                .set_confirm_delete_account_visible(false);
            if let Some(config) = config() {
                let same = config
                    .borrow()
                    .current_account
                    .as_ref()
                    .is_some_and(|account| account.key() == key);
                if same {
                    config.borrow_mut().current_account = None;
                    if let Err(error) = config::save_config(&config.borrow()) {
                        log::warn!("failed to save the config: {error}");
                    }
                }
            }
            ui.global::<GameState>().invoke_refresh();
            on_accounts_changed(&ui);
        });
    });
}

fn skin_action(ui: &App, kind: &str) {
    match kind {
        "save" => save_skin(ui),
        "set" => set_offline_skin(ui),
        "clear" => clear_offline_skin(ui),
        _ => {}
    }
}

fn save_skin(_ui: &App) {
    let Some(account) = VIEWED.with(|cell| cell.borrow().clone()) else {
        return;
    };
    let Some(url) = crate::ui::components::account_avatar::account_skin_url(&account) else {
        return;
    };
    if !url.starts_with("data:") {
        return;
    }
    let Some(path) = crate::ui::services::app_config::save_file_named("Save skin", "skin.png")
    else {
        return;
    };
    crate::support::runtime::spawn(async move {
        if let Err(error) = account::save_skin(url, path.to_string_lossy().to_string()).await {
            log::error!("failed to save the skin: {error}");
        }
    });
}

fn set_offline_skin(ui: &App) {
    let Some(Account::Offline(account)) = VIEWED.with(|cell| cell.borrow().clone()) else {
        return;
    };
    let Some(path) = crate::ui::services::app_config::pick_image_file_named("Select a skin") else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        log::warn!("failed to read {}", path.display());
        return;
    };
    let data = format!("data:image/png;base64,{}", STANDARD.encode(bytes));
    let mut account = account;
    account.skin = Some(data);
    update_offline(ui, account);
}

fn clear_offline_skin(ui: &App) {
    let Some(Account::Offline(account)) = VIEWED.with(|cell| cell.borrow().clone()) else {
        return;
    };
    let mut account = account;
    account.skin = None;
    update_offline(ui, account);
}

fn update_offline(ui: &App, account: OfflineAccount) {
    let key = format!("offline-{}", account.uuid);
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        if let Err(error) = account::offline::update_account(account.clone()).await {
            log::error!("failed to update the offline account: {error}");
            return;
        }
        report(&weak, move |ui| {
            let account = Account::Offline(account);
            VIEWED.with(|cell| {
                if cell.borrow().as_ref().map(Account::key) == Some(key.clone()) {
                    *cell.borrow_mut() = Some(account.clone());
                }
            });
            if let Some(config) = config() {
                let same = config
                    .borrow()
                    .current_account
                    .as_ref()
                    .is_some_and(|current| current.key() == key);
                if same {
                    config.borrow_mut().current_account = Some(account);
                    if let Err(error) = config::save_config(&config.borrow()) {
                        log::warn!("failed to save the config: {error}");
                    }
                }
            }
            ui.global::<GameState>().invoke_refresh();
            refresh(&ui);
        });
    });
}

fn refresh(ui: &App) {
    let state = ui.global::<AccountViewState>();
    let Some(account) = VIEWED.with(|cell| cell.borrow().clone()) else {
        state.set_has_account(false);
        state.set_stats_loaded(false);
        state.set_activities_empty(true);
        state.set_days(ModelRc::new(VecModel::from(Vec::<AccountDay>::new())));
        state.set_months(ModelRc::new(
            VecModel::from(Vec::<AccountMonthLabel>::new()),
        ));
        state.set_activities(ModelRc::new(VecModel::from(Vec::<AccountActivity>::new())));
        return;
    };

    state.set_has_account(true);
    state.set_viewed_key(account.key().into());
    state.set_name(account.get_profile_name().into());
    state.set_kind(account.kind().into());
    state.set_uuid(account.get_profile_uuid().into());
    state.set_auth_server(default_auth_server(&account).into());

    let (kind, year, month, day) = refresh_date(&account);
    state.set_refresh_kind(kind.into());
    state.set_refresh_year(year);
    state.set_refresh_month(month);
    state.set_refresh_day(day);

    state.set_avatar(
        crate::ui::components::account_avatar::account_head(Some(&account), 64).unwrap_or_default(),
    );

    build_skin(ui, &account);
    fetch_server_name(ui, &account);
    fetch_stats(ui, &account);
}

fn default_auth_server(account: &Account) -> String {
    if matches!(account, Account::Yggdrasil(_)) {
        "Yggdrasil".to_string()
    } else {
        String::new()
    }
}

/// The last token refresh, as `("year-month-day" | "never", y, m, d)`.
///
/// A Microsoft access token is minted at the refresh and lives 24 hours, so the
/// refresh moment is the expiry less that lifetime. A Yggdrasil account's token
/// is only ever obtained when the account is added, so its `added_at` is the
/// refresh. An offline account has no auth service at all.
fn refresh_date(account: &Account) -> (&'static str, i32, i32, i32) {
    let seconds = match account {
        Account::Microsoft(account) if account.expires_at > 86_400 => account.expires_at - 86_400,
        Account::Yggdrasil(account) => account.added_at,
        _ => return ("never", 0, 0, 0),
    };
    let Some(date) = Local.timestamp_opt(seconds as i64, 0).single() else {
        return ("never", 0, 0, 0);
    };
    let date = date.date_naive();
    (
        "year-month-day",
        date.year(),
        date.month() as i32,
        date.day() as i32,
    )
}

fn build_skin(ui: &App, account: &Account) {
    let state = ui.global::<AccountViewState>();
    let uuid = account.get_profile_uuid();
    let own_skin = decode_skin(account);

    // The model: the account's own skin when it has a decodable one, otherwise
    // the default skin its UUID picks.
    let (model, has_own_skin) = match own_skin {
        Some(skin) => {
            let slim = slim_for(account, &skin);
            (Some(skin::model_image(&skin, slim)), true)
        }
        None => {
            let image = crate::ui::components::account_avatar::get_default_skin(&uuid)
                .and_then(|default| {
                    crate::ui::components::account_avatar::decode(default.texture_bytes())
                })
                .map(|skin| {
                    let slim = crate::ui::components::account_avatar::get_default_skin(&uuid)
                        .map(|default| default.is_slim())
                        .unwrap_or_else(|| skin::detect_slim(&skin));
                    skin::model_image(&skin, slim)
                });
            (image, false)
        }
    };
    state.set_has_skin(model.is_some());
    state.set_skin_model(model.unwrap_or_default());

    let capes = cape_items(account);
    state.set_capes(ModelRc::new(VecModel::from(capes)));

    // Export needs the skin as a `data:` URL (only a Microsoft profile stores
    // one); setting and clearing a local skin is an offline-account idea.
    state.set_can_save_skin(has_own_skin && is_data_url(account));
    state.set_can_set_skin(matches!(account, Account::Offline(_)));
    state.set_can_clear_skin(matches!(
        account,
        Account::Offline(OfflineAccount { skin: Some(_), .. })
    ));
}

fn is_data_url(account: &Account) -> bool {
    crate::ui::components::account_avatar::account_skin_url(account)
        .is_some_and(|url| url.starts_with("data:"))
}

/// The account's own skin, if it is stored somewhere decodable (a `data:` URL,
/// or an offline account's local file).
fn decode_skin(account: &Account) -> Option<image::RgbaImage> {
    let url = crate::ui::components::account_avatar::account_skin_url(account)?;
    if let Some(image) = crate::ui::components::account_avatar::decode_skin_url(&url) {
        return Some(image);
    }
    // An offline account's skin may be a path on disk.
    if matches!(account, Account::Offline(_)) {
        let path = std::path::Path::new(&url);
        if path.is_file() {
            return std::fs::read(path)
                .ok()
                .and_then(|bytes| crate::ui::components::account_avatar::decode(&bytes));
        }
    }
    None
}

/// The account's capes, each cut to its rounded front face, active one flagged
/// and prefixed by the leading "no cape" entry.
///
/// Only a Microsoft profile carries capes; the other kinds store a remote URL
/// the launcher cannot decode, so they list none. A cape whose texture cannot be
/// decoded is dropped rather than shown as a blank. The "no cape" entry carries
/// no texture; the view draws a ban glyph for it, and it is only present when
/// the account owns at least one cape to take off.
fn cape_items(account: &Account) -> Vec<CapeItem> {
    let Account::Microsoft(account) = account else {
        return Vec::new();
    };
    // The server promises no order and reshuffles the profile on every cape
    // change, so the list is sorted to stay put across reloads: by the cape's
    // own name, then its id.
    let mut capes: Vec<_> = account.profile.capes.iter().collect();
    capes.sort_by(|a, b| a.alias.cmp(&b.alias).then_with(|| a.id.cmp(&b.id)));
    let mut items: Vec<CapeItem> = capes
        .into_iter()
        .filter_map(|cape| {
            let texture = crate::ui::components::account_avatar::decode_skin_url(&cape.url)?;
            let image = skin::cape_image(&texture)?;
            Some(CapeItem {
                image,
                none: false,
                id: cape.id.clone().into(),
                selected: cape.state == "ACTIVE",
            })
        })
        .collect();
    if !items.is_empty() {
        let any_active = items.iter().any(|cape| cape.selected);
        items.insert(
            0,
            CapeItem {
                image: Default::default(),
                none: true,
                id: "".into(),
                selected: !any_active,
            },
        );
    }
    items
}

/// Sets the cape `index` picks on Mojang's side while the view spins: every cape
/// dims and a loader covers the row until the server answers.
///
/// Only a Microsoft cape can be applied (a Yggdrasil profile's textures are its
/// server's to change), and only from a Microsoft account, so anything else is
/// ignored. The account and the list are rebuilt from the server's own answer,
/// so a request that fails leaves the previous selection in place.
fn select_cape(ui: &App, index: i32) {
    let Some(Account::Microsoft(account)) = VIEWED.with(|cell| cell.borrow().clone()) else {
        return;
    };
    let state = ui.global::<AccountViewState>();
    let Some(chosen) = state.get_capes().row_data(index.max(0) as usize) else {
        return;
    };
    if chosen.selected {
        return;
    }
    let cape_id = chosen.id.clone();
    let none = chosen.none;
    let uuid = account.profile.uuid;

    state.set_cape_loading(true);

    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let result = if none {
            account::microsoft::clear_active_cape(uuid).await
        } else {
            account::microsoft::set_active_cape(uuid, &cape_id).await
        };
        report(&weak, move |ui| {
            let state = ui.global::<AccountViewState>();
            state.set_cape_loading(false);
            // Adopt the server's answer only while this account is still the one
            // being shown; a switch in the meantime must not have its list
            // overwritten.
            let key = format!("microsoft-{uuid}");
            let applied = match result {
                Ok(updated) => {
                    let updated = Account::Microsoft(updated);
                    let showing = VIEWED.with(|cell| {
                        if cell.borrow().as_ref().map(Account::key) == Some(key) {
                            *cell.borrow_mut() = Some(updated.clone());
                            true
                        } else {
                            false
                        }
                    });
                    showing.then_some(updated)
                }
                Err(error) => {
                    log::error!("failed to change the active cape: {error}");
                    None
                }
            };
            // Rebuild from the account being shown, whether the request changed
            // it (the reloaded profile) or not (the previous selection).
            let account = applied.or_else(|| VIEWED.with(|cell| cell.borrow().clone()));
            if let Some(account) = account {
                state.set_capes(ModelRc::new(VecModel::from(cape_items(&account))));
            }
        });
    });
}

/// Whether a skin texture is a slim model, using the account's own metadata and
/// falling back to the arm's pixel width.
fn slim_for(account: &Account, skin: &image::RgbaImage) -> bool {
    if let Account::Microsoft(account) = account
        && let Some(skin) = account.profile.skins.first()
    {
        return skin.variant == "slim";
    }
    skin::detect_slim(skin)
}

fn fetch_server_name(ui: &App, account: &Account) {
    let Account::Yggdrasil(account) = account else {
        return;
    };
    let api_root = account.api_root.clone();
    let key = format!("yggdrasil-{}", account.identifier);
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let info = account::yggdrasil::yggdrasil_server::get_server_info(&api_root).await;
        report(&weak, move |ui| {
            let Ok(info) = info else { return };
            let state = ui.global::<AccountViewState>();
            if state.get_viewed_key().as_str() != key.as_str() {
                return;
            }
            if let Some(name) = info.meta.get("serverName").and_then(|name| name.as_str()) {
                state.set_auth_server(name.into());
            }
        });
    });
}

/// The statistics profile a stored account logs its launches under.
fn statistics_profile(account: &Account) -> StatisticsProfile {
    match account {
        Account::Microsoft(account) => StatisticsProfile::Microsoft(account.profile.uuid),
        Account::Offline(account) => StatisticsProfile::Offline(account.uuid),
        Account::Yggdrasil(account) => StatisticsProfile::Yggdrasil(account.identifier),
    }
}

fn fetch_stats(ui: &App, account: &Account) {
    let profile = statistics_profile(account);
    let key = account.key();
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let entries = statistics::get_statistics_by_profile(profile)
            .await
            .unwrap_or_default();
        let names: HashMap<String, String> = instance::list_instances(instance::SortBy::Playtime)
            .await
            .map(|instances| {
                instances
                    .into_iter()
                    .map(|instance| (instance.id, instance.config.name))
                    .collect()
            })
            .unwrap_or_default();
        report(&weak, move |ui| {
            if ui.global::<AccountViewState>().get_viewed_key().as_str() != key.as_str() {
                return;
            }
            apply_stats(&ui, &entries, &names);
        });
    });
}

/// Builds the 1-year contribution window, the month captions and the recent
/// activity rows. Ported from the Vue build's `ActivityCalendar`.
fn apply_stats(ui: &App, entries: &[StatisticsEntry], names: &HashMap<String, String>) {
    let state = ui.global::<AccountViewState>();

    let today = Local::now().date_naive();
    let year = today.year();
    // The same month/day a year ago, a day back when the month has no such day
    // (Feb 29), matching JavaScript's `setFullYear` overflow.
    let start = NaiveDate::from_ymd_opt(year - 1, today.month(), today.day())
        .or_else(|| NaiveDate::from_ymd_opt(year - 1, today.month(), today.day() - 1))
        .unwrap_or(today - Duration::days(365));
    // Pad the window back to the Sunday that opens the first column, so that
    // column is a full week rather than the one to six days the year boundary
    // happens to leave. GitHub does the same: the padded days are real days of
    // the window at level 0, not blank cells.
    let leading = start.weekday().num_days_from_sunday() as i64;
    let start = start - Duration::days(leading);
    let days = (today - start).num_days().max(0) as usize + 1;

    let mut counts = vec![0u32; days];
    for entry in entries {
        let Some(date) = Local
            .timestamp_opt(entry.launch_at_unix_secs as i64, 0)
            .single()
            .map(|date| date.date_naive())
        else {
            continue;
        };
        let offset = (date - start).num_days();
        if offset < 0 || offset as usize >= days {
            continue;
        }
        counts[offset as usize] += 1;
    }

    let cells: Vec<AccountDay> = counts
        .iter()
        .enumerate()
        .map(|(index, count)| AccountDay {
            level: level(*count),
            column: index as i32 / 7,
            row: index as i32 % 7,
        })
        .collect();

    let column_count = days.div_ceil(7) as i32;

    state.set_total_launches(counts.iter().sum::<u32>() as i32);
    state.set_column_count(column_count);
    state.set_days(ModelRc::new(VecModel::from(cells)));
    state.set_months(ModelRc::new(VecModel::from(month_labels(
        start,
        column_count,
    ))));
    state.set_stats_loaded(true);

    let mut recent: Vec<&StatisticsEntry> = entries.iter().collect();
    recent.sort_by_key(|entry| std::cmp::Reverse(entry.launch_at_unix_secs));
    let activities: Vec<AccountActivity> = recent
        .into_iter()
        .take(30)
        .map(|entry| {
            let name = names
                .get(&entry.instance_id)
                .cloned()
                .unwrap_or_else(|| entry.instance_id.clone());
            let time =
                crate::ui::views::game::relative_time(Some(entry.launch_at_unix_secs * 1000));
            AccountActivity {
                name: name.into(),
                kind: time.kind.into(),
                hours: time.hours,
                month: time.month,
                day: time.day,
                year: time.year,
            }
        })
        .collect();
    state.set_activities_empty(activities.is_empty());
    state.set_activities(ModelRc::new(VecModel::from(activities)));
}

/// A launch count's GitHub-style level, 0 through 4.
fn level(count: u32) -> i32 {
    match count {
        0 => 0,
        1..=2 => 1,
        3..=5 => 2,
        6..=10 => 3,
        _ => 4,
    }
}

/// The month captions: a label on any run of two or more columns that share a
/// month.
fn month_labels(start: NaiveDate, columns: i32) -> Vec<AccountMonthLabel> {
    let month_of = |column: i32| -> i32 {
        let column_start = start + Duration::days(column as i64 * 7);
        column_start.year() * 12 + column_start.month() as i32
    };

    let mut labels = Vec::new();
    let mut run_start = 0;
    let mut run_month = month_of(0);
    for column in 1..=columns {
        let next = if column < columns {
            month_of(column)
        } else {
            -1
        };
        if next != run_month {
            if column - run_start >= 2 {
                labels.push(AccountMonthLabel {
                    column: run_start,
                    month: run_month.rem_euclid(12) + 1,
                });
            }
            run_start = column;
            run_month = next;
        }
    }
    labels
}
