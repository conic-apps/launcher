// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The game view's script: drives the Rust domain crates and pushes the results
//! into the `GameState` global.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    time::{Duration, SystemTime},
};

use chrono::{Datelike, Local, TimeZone};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::slint_backend::{
    AccountItem, App, Dialogs, GameRow, GameState, InstanceMenu, MultiplayerState, TaskState,
};
use account::Account;
use instance::{Instance, ModLoaderType, SortBy};
use slint::Image;

use crate::usecases::game::{GroupMode, InstanceViewState, SortMode as SavedSortMode};

mod controller;
pub(crate) mod install;
mod install_progress;
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

pub(crate) fn controller() -> Rc<RefCell<GameController>> {
    CONTROLLER
        .with(|cell| cell.borrow().clone())
        .expect("the game controller is set up before any of its callbacks run")
}

pub(crate) struct RelativeTime {
    pub(crate) kind: &'static str,
    pub(crate) hours: i32,
    pub(crate) month: i32,
    pub(crate) day: i32,
    pub(crate) year: i32,
}

/// A list row's decoded instance background, with the file revision it was
/// decoded from so a replaced or removed image is picked up on the next apply.
pub(crate) struct CachedBackground {
    modified: Option<SystemTime>,
    image: Image,
}

/// The instance backgrounds the list draws, and the decodes still running for
/// the ones it cannot draw yet.
///
/// A decode is not a list change: an instance background is tens of megabytes
/// and takes hundreds of milliseconds, while `apply` runs on the event loop and
/// on every keystroke in the search field. So the row is built without the
/// picture and the picture is dropped into it when it lands.
#[derive(Default)]
pub(crate) struct BackgroundCache {
    images: HashMap<String, CachedBackground>,
    /// The instances whose decode is in flight, so a second apply cannot start a
    /// second decode of the same picture.
    decoding: HashSet<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct GroupRange {
    key: String,
    expanded: bool,
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
    /// Memoised because a skin is a base64 PNG (or a bundled webp) that has to
    /// be decoded and cropped, and `apply` runs on every list change.
    avatars: HashMap<String, Image>,
    /// Memoised by the background file's modified time, which is the
    /// invalidation key. See [`BackgroundCache`].
    backgrounds: RefCell<BackgroundCache>,
    /// Cached so a wheel event can decide whether it crossed a group edge
    /// without rebuilding the rows.
    groups: Vec<GroupRange>,
    /// Kept across applies and reconciled in place (see `sync_rows`), so the
    /// view's row items survive a relayout and can animate along the rail to
    /// their new slot instead of being recreated.
    rows_model: Rc<VecModel<GameRow>>,
    reveal_timer: Timer,
    /// Whether the list has been handed to the view at least once. That first
    /// layout is what the view's intro slides in, so it starts revealed and only
    /// rows added later use the appear animation.
    synced: bool,
    /// Whether rows animate to their new slots on the 400ms FLIP (`true`) or the
    /// 300ms collapse curve (`false`); reset to `true` by `apply`.
    flip: bool,
}

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

pub(crate) fn format_decimal(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// Shared with the content overlays' saves cards (`content`).
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
