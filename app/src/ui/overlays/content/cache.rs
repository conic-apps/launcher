// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance's local content, parsed once and read by everything that shows
//! it, so the summary's counts and the overlay's list cannot disagree. A stale
//! entry is served while it is replaced, and only the current instance is written
//! to the globals.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use content::resourcepack::Resourcepack;
use content::saves::LevelSummary;

use super::*;

/// A floor on how often the disk is walked, so a file dropped in by hand shows
/// up without a reload. The app's own writes go through [`refresh`] instead.
const CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Kind {
    Saves,
    Mods,
    ResourcePacks,
    Screenshots,
}

impl Kind {
    /// In the order the summary lists them and the preview models are numbered.
    pub(crate) const ALL: [Kind; 4] = [
        Kind::Saves,
        Kind::Mods,
        Kind::ResourcePacks,
        Kind::Screenshots,
    ];
}

/// The data is behind an `Arc` because every projection is handed a handle
/// rather than a copy — a two-hundred-mod list is a few hundred structs, and the
/// summary, the overlay and the preview row all want it in the same frame — and
/// because a projection's icon work runs on the runtime, where only an `Arc`
/// goes.
struct Cached<T> {
    data: Arc<Vec<T>>,
    at: Instant,
}

/// One instance's four lists. A kind is `None` until it has been parsed once —
/// which is what the "nothing loaded yet" and "parsed, and there is none" cases
/// are told apart by, the same `null`-versus-empty split the original store used.
#[derive(Default)]
struct InstanceContent {
    saves: Option<Cached<(String, LevelSummary)>>,
    mods: Option<Cached<ResolvedMod>>,
    resourcepacks: Option<Cached<Resourcepack>>,
    screenshots: Option<Cached<PathBuf>>,
}

impl InstanceContent {
    fn parsed_at(&self, kind: Kind) -> Option<Instant> {
        match kind {
            Kind::Saves => self.saves.as_ref().map(|cached| cached.at),
            Kind::Mods => self.mods.as_ref().map(|cached| cached.at),
            Kind::ResourcePacks => self.resourcepacks.as_ref().map(|cached| cached.at),
            Kind::Screenshots => self.screenshots.as_ref().map(|cached| cached.at),
        }
    }
}

enum Parsed {
    Saves(Vec<(String, LevelSummary)>),
    Mods(Vec<ResolvedMod>),
    ResourcePacks(Vec<Resourcepack>),
    Screenshots(Vec<PathBuf>),
}

thread_local! {
    /// The parsed lists, by instance id. Every access is on the UI thread, which
    /// is where the views are and where a parse reports back to.
    static CACHE: RefCell<HashMap<String, InstanceContent>> = RefCell::new(HashMap::new());
    /// How many parses of each kind are running, by instance. The summary's one
    /// loading flag is true while any of the four is out.
    static INFLIGHT: RefCell<HashMap<(String, Kind), usize>> = RefCell::new(HashMap::new());
    /// The parse number handed to each started parse. A result whose number has
    /// been overtaken is dropped rather than filed, which is what stops a
    /// re-parse started after an install from losing to the parse that was
    /// already running when the install happened.
    static SEQ: RefCell<HashMap<(String, Kind), u64>> = RefCell::new(HashMap::new());
}

pub(crate) fn ensure(ui: &App, instance: &str) {
    for kind in Kind::ALL {
        start(ui, instance, kind, true);
    }
    loading(ui, instance);
}

/// Forgets one kind of one instance, so the next read re-parses it.
///
/// This is how the app's own writes — a mod installed, a mod removed, a save
/// deleted — reach both views at once, rather than each of them reloading what
/// it happens to be showing.
pub(crate) fn refresh(ui: &App, instance: &str, kind: Kind) {
    forget(instance, kind);
    start(ui, instance, kind, false);
    loading(ui, instance);
}

/// Every parsed list, dropped.
///
/// Called when the instance list is re-read, which is the one moment the app
/// knows the disk moved underneath it wholesale.
pub(crate) fn clear() {
    CACHE.with(|cache| cache.borrow_mut().clear());
}

pub(crate) fn saves(instance: &str) -> Option<Arc<Vec<(String, LevelSummary)>>> {
    CACHE.with(|cache| {
        cache
            .borrow()
            .get(instance)
            .and_then(|entry| entry.saves.as_ref())
            .map(|cached| Arc::clone(&cached.data))
    })
}

/// The instance's mods, jar-in-jar ones already filtered out — the store decides
/// that, so the count on the summary and the cards in the panel can never
/// disagree about what a mod is.
pub(crate) fn mods(instance: &str) -> Option<Arc<Vec<ResolvedMod>>> {
    CACHE.with(|cache| {
        cache
            .borrow()
            .get(instance)
            .and_then(|entry| entry.mods.as_ref())
            .map(|cached| Arc::clone(&cached.data))
    })
}

pub(crate) fn resourcepacks(instance: &str) -> Option<Arc<Vec<Resourcepack>>> {
    CACHE.with(|cache| {
        cache
            .borrow()
            .get(instance)
            .and_then(|entry| entry.resourcepacks.as_ref())
            .map(|cached| Arc::clone(&cached.data))
    })
}

pub(crate) fn screenshots(instance: &str) -> Option<Arc<Vec<PathBuf>>> {
    CACHE.with(|cache| {
        cache
            .borrow()
            .get(instance)
            .and_then(|entry| entry.screenshots.as_ref())
            .map(|cached| Arc::clone(&cached.data))
    })
}

/// Starts a parse of one kind. `deduped` is [`ensure`]'s case — it runs on every
/// game view apply, so a search keystroke must not start a second parse over the
/// same folder; [`refresh`] passes `false`, because it follows a write.
fn start(ui: &App, instance: &str, kind: Kind, deduped: bool) {
    let key = (instance.to_string(), kind);
    // A fresh entry answers on its own, and so does one already being parsed.
    // `refresh` is exempt from the first because it has just invalidated the
    // entry on purpose — that is the whole of what it is for.
    if deduped && (fresh(instance, kind) || is_running(&key)) {
        return;
    }
    let seq = SEQ.with(|seq| {
        let mut seq = seq.borrow_mut();
        let next = seq.entry(key.clone()).or_default();
        *next += 1;
        *next
    });
    INFLIGHT.with(|inflight| {
        *inflight.borrow_mut().entry(key.clone()).or_default() += 1;
    });
    let weak = ui.as_weak();
    let instance = instance.to_string();
    crate::support::runtime::spawn(async move {
        let parsed = parse(&instance, kind).await;
        crate::ui::services::report::report(&weak, move |ui| {
            if land(&instance, kind, seq) {
                file_parsed(&instance, parsed);
                // The summary's counts are the cache's own lengths, so they go up
                // now that there is something to count; the icons follow on their own.
                // The open panel of this kind is redrawn from the same list.
                counts(&ui, Some(&instance));
                preview::refresh_preview_icons(&ui, &instance);
                repanel(&ui, &instance, kind);
            }
            loading(&ui, &instance);
        });
    });
}

/// Redraws the panel that is open, if it is this kind's.
///
/// Only the open one: a grid is laid out at the width of the panel showing it,
/// and projecting into a closed one would take over the grid a window resize
/// re-lays out. A closed panel draws from the cache when it opens instead.
fn repanel(ui: &App, instance: &str, kind: Kind) {
    if ui.global::<GameState>().get_current_id() != instance {
        return;
    }
    let open = ui.global::<ContentState>().get_open_panel().to_string();
    match (kind, open.as_str()) {
        (Kind::Saves, "saves") => project_saves(ui),
        (Kind::Mods, "mods") => project_local_mods(ui),
        (Kind::ResourcePacks, "resourcepacks") => project_local_resourcepacks(ui),
        (Kind::Screenshots, "screenshots") => project_screenshots(ui),
        _ => {}
    }
}

fn is_running(key: &(String, Kind)) -> bool {
    INFLIGHT.with(|inflight| inflight.borrow().get(key).is_some_and(|count| *count > 0))
}

/// Whether the cache still answers for this kind without going to the disk.
///
/// This is the only place [`CACHE_TTL`] is read, which is what makes the accessors
/// able to ignore it: they hand out a stale list rather than nothing, and this
/// decides when that list stops being the answer.
fn fresh(instance: &str, kind: Kind) -> bool {
    CACHE.with(|cache| {
        cache
            .borrow()
            .get(instance)
            .and_then(|entry| entry.parsed_at(kind))
            .is_some_and(|at| at.elapsed() < CACHE_TTL)
    })
}

/// Files `parsed` and reports back whether it was still the newest one.
///
/// A parse that has been overtaken is dropped: the one that replaced it read the
/// folder later, so it is the one that saw what is actually on disk.
fn land(instance: &str, kind: Kind, seq: u64) -> bool {
    let key = (instance.to_string(), kind);
    let current = SEQ.with(|seq| seq.borrow().get(&key).copied());
    INFLIGHT.with(|inflight| {
        let mut inflight = inflight.borrow_mut();
        if let Some(count) = inflight.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                inflight.remove(&key);
            }
        }
    });
    current == Some(seq)
}

fn file_parsed(instance: &str, parsed: Parsed) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let entry = cache.entry(instance.to_string()).or_default();
        let at = Instant::now();
        match parsed {
            Parsed::Saves(mut data) => {
                // By folder, so the summary's five previews and the panel's
                // list are the same worlds in the same order.
                data.sort_by(|(left, _), (right, _)| left.cmp(right));
                entry.saves = Some(Cached {
                    data: Arc::new(data),
                    at,
                });
            }
            Parsed::Mods(data) => {
                entry.mods = Some(Cached {
                    data: Arc::new(data),
                    at,
                })
            }
            Parsed::ResourcePacks(data) => {
                entry.resourcepacks = Some(Cached {
                    data: Arc::new(data),
                    at,
                });
            }
            Parsed::Screenshots(data) => {
                entry.screenshots = Some(Cached {
                    data: Arc::new(data),
                    at,
                })
            }
        }
    });
}

fn forget(instance: &str, kind: Kind) {
    CACHE.with(|cache| {
        if let Some(entry) = cache.borrow_mut().get_mut(instance) {
            match kind {
                Kind::Saves => entry.saves = None,
                Kind::Mods => entry.mods = None,
                Kind::ResourcePacks => entry.resourcepacks = None,
                Kind::Screenshots => entry.screenshots = None,
            }
        }
    });
}

/// Reads one kind off the disk. Everything that can block does.
async fn parse(instance: &str, kind: Kind) -> Parsed {
    match kind {
        // Gzip and NBT parsing is blocking, so it runs on the blocking pool.
        Kind::Saves => {
            let levels = crate::support::runtime::spawn_blocking({
                let instance = instance.to_string();
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
            // The aborted-or-panicked task and the failed read carry different error
            // types, so they are logged separately rather than chained.
            .map(|result| {
                result.unwrap_or_else(|error| {
                    log::warn!("The saves of {instance} could not be read: {error}");
                    Vec::new()
                })
            })
            .unwrap_or_else(|error| {
                log::warn!("Reading the saves of {instance} did not run: {error}");
                Vec::new()
            });
            Parsed::Saves(levels)
        }
        // Jar-in-jar mods are hidden here rather than in each projection. They
        // are one entry inside another mod's jar, not a file of their own, so
        // letting them through would make the summary count a mod the panel
        // cannot open.
        Kind::Mods => Parsed::Mods(
            content::remote::mods::parse_mods(instance)
                .await
                .into_iter()
                .filter(|mod_info| !mod_info.embedded)
                .collect(),
        ),
        // The local parse and the platform lookup share the resource-pack cache.
        Kind::ResourcePacks => Parsed::ResourcePacks(
            content::remote::resourcepacks::parse_resourcepacks(instance).await,
        ),
        Kind::Screenshots => Parsed::Screenshots(
            content::screenshots::list_screenshots(instance)
                .unwrap_or_default()
                .into_iter()
                .map(PathBuf::from)
                .collect(),
        ),
    }
}

/// The summary's four counts, taken from the cache: a count is `len()` of the list
/// the panel draws, so there is no second notion of "what counts as a mod" to
/// disagree. `None` is the summary with no instance behind it; an instance that is
/// not the current one is left alone.
pub(crate) fn counts(ui: &App, instance: Option<&str>) {
    let state = ui.global::<GameState>();
    let instance = match instance {
        Some(id) if state.get_current_id() == id => id,
        Some(_) => return,
        None => {
            state.set_content_saves(0);
            state.set_content_mods(0);
            state.set_content_resourcepacks(0);
            state.set_content_screenshots(0);
            state.set_content_loading_saves(false);
            state.set_content_loading_mods(false);
            state.set_content_loading_resourcepacks(false);
            state.set_content_loading_screenshots(false);
            return;
        }
    };
    let len = |count: Option<usize>| count.unwrap_or_default() as i32;
    state.set_content_saves(len(saves(instance).map(|list| list.len())));
    state.set_content_mods(len(mods(instance).map(|list| list.len())));
    state.set_content_resourcepacks(len(resourcepacks(instance).map(|list| list.len())));
    state.set_content_screenshots(len(screenshots(instance).map(|list| list.len())));
    state.set_content_loading_saves(parsing(instance, Kind::Saves));
    state.set_content_loading_mods(parsing(instance, Kind::Mods));
    state.set_content_loading_resourcepacks(parsing(instance, Kind::ResourcePacks));
    state.set_content_loading_screenshots(parsing(instance, Kind::Screenshots));
}

/// Whether one kind of `instance` is being parsed right now. One flag each,
/// because the four settle at very different times: a screenshot folder is a
/// `read_dir`, a mods folder is a SHA-512 over every jar plus two platform lookups.
fn parsing(instance: &str, kind: Kind) -> bool {
    INFLIGHT.with(|inflight| {
        inflight
            .borrow()
            .get(&(instance.to_string(), kind))
            .is_some_and(|count| *count > 0)
    })
}

/// The panels' own four spinners, which follow the same parses.
///
/// Written even for an instance the summary is not showing, because the overlay
/// is told which instance it is on only when a panel opens (`sync_instance`) and
/// the flag has to describe that one.
fn loading(ui: &App, instance: &str) {
    // The flags belong to whatever the overlay is showing, which is the game
    // view's current instance. A parse that outlived the selection must not
    // clear the spinner of the one that is on screen.
    if ui.global::<GameState>().get_current_id() != instance {
        return;
    }
    let state = ui.global::<ContentState>();
    state.set_saves_loading(parsing(instance, Kind::Saves));
    state.set_local_mods_loading(parsing(instance, Kind::Mods));
    state.set_local_resourcepacks_loading(parsing(instance, Kind::ResourcePacks));
    state.set_screenshots_loading(parsing(instance, Kind::Screenshots));
}
