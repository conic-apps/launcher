// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's script: drives the Rust domain crates and pushes the results
//! into the `GameState` global.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use chrono::{Datelike, Local, TimeZone};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::slint_backend::{
    AccountItem, App, Dialogs, GameRow, GameState, MultiplayerState, Navigation,
};
use account::Account;
use instance::{Instance, ModLoaderType, SortBy};
use slint::Image;

use crate::instance_view::{self, GroupMode, InstanceViewState, SortMode as SavedSortMode};

mod controller;
mod wiring;

pub(crate) use wiring::*;

thread_local! {
    /// The one controller, for use on the UI thread.
    ///
    /// It lives in a thread-local rather than in a value `setup` owns because an
    /// `upgrade_in_event_loop` closure has to be `Send`, and the `Rc<RefCell<…>>`
    /// a Slint model is cannot cross into one. Slint's callbacks and the
    /// event-loop closures both run on the UI thread, so both reach it through
    /// [`controller`]; the async tasks carry only owned values and a `Weak<App>`.
    static CONTROLLER: RefCell<Option<Rc<RefCell<GameController>>>> =
        const { RefCell::new(None) };
}

/// The controller, for use on the UI thread.
pub(crate) fn controller() -> Rc<RefCell<GameController>> {
    CONTROLLER
        .with(|cell| cell.borrow().clone())
        .expect("the game controller is set up before any of its callbacks run")
}

/// A ready-to-display relative time (`GameTime.last-played`).
pub(crate) struct RelativeTime {
    pub(crate) kind: &'static str,
    pub(crate) hours: i32,
    pub(crate) month: i32,
    pub(crate) day: i32,
    pub(crate) year: i32,
}

pub(crate) struct GameController {
    config: Rc<RefCell<config::Config>>,
    instances: Vec<Instance>,
    current_id: Option<String>,
    sort: SortBy,
    group_mode: &'static str,
    search: String,
    expanded: HashMap<String, bool>,
    accounts: Vec<Account>,
    playtime: HashMap<String, u64>,
    content: HashMap<String, content::ContentCounts>,
    /// The account heads the footer and its switcher draw, by `<key>@<size>`.
    /// Memoised because a skin is a base64 PNG (or a bundled webp) that has to
    /// be decoded and cropped, and `apply` runs on every list change.
    avatars: HashMap<String, Image>,
    /// The instance list. It is kept across applies and reconciled in place (see
    /// `sync_rows`), so the view's row items survive a relayout and can animate
    /// along the rail to their new slot instead of being recreated in place.
    rows_model: Rc<VecModel<GameRow>>,
    /// Reveals the rows that `sync_rows` added, 50ms after they appeared.
    reveal_timer: Timer,
    /// Whether the list has been handed to the view at least once. The rows of
    /// that first layout are the ones the view's intro slides in, so they start
    /// revealed and only rows added later use the appear animation.
    synced: bool,
    /// Whether the rows about to be laid out animate to their new slots on the
    /// 400ms FLIP (`true`) or the 300ms `cubic-bezier(0.33, 1, 0.68, 1)` collapse
    /// curve (`false`). Reset to `true` by `apply`.
    flip: bool,
}

/// Reveals the rows added by the last `sync_rows`: `appear` flipping to true is
/// what makes the view animate them in along the rail.
pub(crate) fn reveal_rows(model: &VecModel<GameRow>) {
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
pub(crate) fn format_play_time(seconds: u64) -> (&'static str, String) {
    if seconds < 60 {
        return ("seconds", seconds.to_string());
    }
    let minutes = seconds as f64 / 60.0;
    if minutes < 60.0 {
        return ("minutes", format_decimal(minutes));
    }
    ("hours", format_decimal(minutes / 60.0))
}

/// Rounds to one decimal and strips a trailing `.0`.
pub(crate) fn format_decimal(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// Resolves the relative-time parts of a last-played timestamp. Shared with the
/// content overlays' saves cards (`content`).
pub(crate) fn relative_time(timestamp: Option<u64>) -> RelativeTime {
    let Some(timestamp) = timestamp else {
        return RelativeTime {
            kind: "never",
            hours: 0,
            month: 0,
            day: 0,
            year: 0,
        };
    };
    // The launch script stores the timestamp in milliseconds.
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
