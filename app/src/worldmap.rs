// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world map's "script": the tile cache and the render queue behind
//! `ui/components/world-map.slint`.
//!
//! # The Vue's two caches, and the one here
//!
//! `WorldMap.vue` kept two, and both existed because of the IPC boundary:
//!
//!   * `pngCache` — every tile's **PNG bytes**, kept for the whole session so a
//!     tile that scrolled away and came back did not have to be rendered again.
//!     Base64'd into a JSON string on the way out.
//!   * `renderCache` — the **decoded `ImageBitmap`s**, pruned to the visible
//!     range plus `TILE_RENDER_MARGIN` rings, because a decoded bitmap is what
//!     the GPU holds and there is a limit on how many.
//!
//! and a third queue for the work in between: PNG bytes had to be base64'd,
//! wrapped in a `Blob` and decoded by `createImageBitmap`, one tile at a time,
//! off the main thread but still inside the page.
//!
//! None of that survives the move. A Slint `Image` is a reference-counted handle
//! over a `SharedPixelBuffer`: cloning one is free, there is no encoded form to
//! keep a second copy of, and the renderer drops its own GPU copy under memory
//! pressure on its own. So there is **one** cache, here, keyed by tile and
//! holding the finished `Image`, and the two tiers collapse into "cached" and
//! "not cached yet". The model the component draws is a *view* of it — the tiles
//! in the visible range — and it is written row by row, so a tile keeps its
//! element for as long as it is on screen.
//!
//! # The load queue
//!
//! `WorldMap.vue` had a `pending` set, an `inFlight` set, a nearest-to-centre
//! priority scan and a 50ms debounce, all driven from JavaScript. The same four
//! live here, where the queue actually is, and the debounce keeps the Vue's
//! exact shape: the model is refilled from the cache **immediately** when the
//! range moves (`dispatchCacheHits`, which ran outside the timer) and only the
//! *renders* are debounced.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use slint::{ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel};

use crate::slint_backend::{App, ScrollInput, WorldMapState, WorldSource, WorldTile};
use content::worldmap::{MapCache, WorldMapRequest};

/// How many tiles may be rendering at once (`MAX_CONCURRENT`).
///
/// The Vue's number, and for its reason: a render walks every block column in a
/// 64×64 tile, so a few dozen in flight saturate the disk and the CPU and
/// everything past that is only queueing.
const MAX_CONCURRENT: usize = 24;

/// The Vue's `TILE_LOAD_DEBOUNCE`. A drag or a zoom glide moves the visible
/// range many times a second, and every move would otherwise queue a fresh
/// batch of renders for a range the user is about to leave.
const LOAD_DEBOUNCE: Duration = Duration::from_millis(50);

/// The Vue's `FADE_MS`, which the component's own fade clock counts down. Rust
/// only needs it to publish how long the fade window stays open.
const FADE_MS: f32 = 100.0;

/// How many tiles the cache holds before it starts letting go. A cap the Vue
/// did not need: its `pngCache` held *compressed* bytes, a twentieth of what a
/// decoded tile costs, so a session's worth of panning added up to something
/// nobody noticed. A 64×64 RGBA tile is 16 KiB, and a map the user has panned
/// right across is thousands of them — 2048 is about 32 MiB, and roughly twenty
/// screens of the default zoom, which is what makes coming back to where you
/// were a hit rather than a re-render.
///
/// What is dropped is re-rendered on demand, off the region cache the world
/// keeps alive in `MapCache` — which is exactly what the Vue paid for a
/// `pngCache` *miss*, and the price of its unbounded tier.
const MAX_CACHED_TILES: usize = 2048;

/// How far outside the visible range a queued render is kept.
///
/// A drag or a zoom glide moves the range many times a second, and without this
/// the queue would keep every tile of every range it passed through: a long pan
/// leaves hundreds of jobs for tiles that are off screen, all of them competing
/// for the 24 slots with the ones the user is actually looking at. A tile that
/// leaves by more than this is very unlikely to be wanted again, and dropping it
/// from the queue costs nothing — the cache is what remembers it.
const QUEUE_MARGIN: i32 = 8;

/// A tile's position in the tile grid: which square of the world it is. This is
/// the key of the cache, of the model and of the queue, and it is what the
/// component multiplies by the tile size to place a tile.
type TileKey = (i32, i32);

/// The world a tile belongs to, and how it is rendered. The cache's own key: a
/// different save is a different map and its tiles mean nothing here. This is
/// the `WorldSource` the component reports, reduced to what a cache key needs.
#[derive(Clone, PartialEq, Eq)]
struct WorldKey {
    instance_id: String,
    folder: String,
    dimension: String,
    tile_size: u32,
}

/// What one tile of the queue needs to be rendered.
#[derive(Clone)]
struct Job {
    key: TileKey,
    /// The world block at the tile's centre — a render request is *centred*
    /// (`RenderRequest::new`), the same way the Vue asked for
    /// `centerX: (tx + 0.5) * tileSize`.
    center_x: i32,
    center_z: i32,
    /// Which world the job belongs to. Checked on arrival, so a render that
    /// lands after the user has picked a different save is dropped: the Vue's
    /// `if (seq !== requestSeq) return`.
    seq: u64,
}

struct MapState {
    /// The world on screen. `None` until a map reports one.
    world: Option<WorldKey>,
    /// The render options the component declared, which every tile is asked for.
    options: RenderOptions,
    /// The `conic-worldmap` worlds, kept alive so a tile that scrolls back into
    /// view re-reads a warm region cache instead of the disk — the crate's own
    /// `MapCache`, and the one half of the Vue's caching that did not move.
    maps: Arc<MapCache>,
    /// Every tile rendered for `world`, by position. The `pngCache` and the
    /// `renderCache` merged: there is nothing cheaper to keep than the finished
    /// image, and nothing to decode.
    cache: HashMap<TileKey, Image>,
    /// When each cached tile was last inside a visible range, as a counter off
    /// [`MapState::tick`]. The eviction order — see [`prune_cache`].
    touched: HashMap<TileKey, u32>,
    tick: u32,
    /// The tiles in the model, in the order they were added. A tile leaves when
    /// it is dropped from here, and the model's rows are index-aligned with it.
    visible: Vec<TileKey>,
    /// When each tile first reached the model, for the fade. Kept after the tile
    /// has faded — which is what stops a pan back over cached ground from
    /// flashing, where the Vue's `tileAppear` entry was deleted by
    /// `pruneRenderCache` and never set again for a cache hit.
    appear: HashMap<TileKey, f32>,
    /// The model itself, held so rows can be rewritten in place.
    model: Rc<VecModel<WorldTile>>,
    /// The range the component last asked for, and the middle of it, which is
    /// what the queue orders by.
    range: Option<Range>,
    /// Tiles waiting to be rendered, nearest the middle of the range first.
    queue: Vec<Job>,
    /// Tiles being rendered right now.
    in_flight: HashSet<TileKey>,
    /// Tiles a render failed on. The Vue's `tilesFailed`, which is what stopped
    /// it asking again for a region that cannot be read.
    failed: HashSet<TileKey>,
    /// The debounce handle. Restarting it is the debounce; aborting it drops a
    /// batch the user has already panned away from.
    debounce: Option<tokio::task::JoinHandle<()>>,
    /// Bumped on every world change, so a render that lands late is dropped.
    seq: u64,
}

/// The render toggles `RenderOptions` carries, kept in one place because every
/// tile of a world is asked for the same ones.
#[derive(Clone, Copy, PartialEq, Eq)]
struct RenderOptions {
    water: bool,
    shading: bool,
    altitude_shading: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            water: true,
            shading: true,
            altitude_shading: true,
        }
    }
}

/// An inclusive tile range: `x0..=x1` by `z0..=z1`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Range {
    x0: i32,
    x1: i32,
    z0: i32,
    z1: i32,
}

impl Range {
    /// Every tile of the range, row by row — the loops the Vue's
    /// `visibleTileRange()` is walked with.
    fn tiles(&self) -> impl Iterator<Item = TileKey> + '_ {
        (self.z0..=self.z1).flat_map(|tz| (self.x0..=self.x1).map(move |tx| (tx, tz)))
    }

    /// The middle of the range in world blocks — the Vue's
    /// `(minX + maxX) / 2` of `visibleWorldRect`, which is what the queue
    /// measures a tile's distance from.
    fn centre(&self, tile_size: i32) -> (f64, f64) {
        let size = f64::from(tile_size);
        (
            (f64::from(self.x0) + f64::from(self.x1)) / 2.0 * size + size / 2.0,
            (f64::from(self.z0) + f64::from(self.z1)) / 2.0 * size + size / 2.0,
        )
    }

    fn holds(&self, key: TileKey) -> bool {
        key.0 >= self.x0 && key.0 <= self.x1 && key.1 >= self.z0 && key.1 <= self.z1
    }
}

thread_local! {
    static STATE: RefCell<MapState> = RefCell::new(MapState::new());
}

impl MapState {
    fn new() -> Self {
        Self {
            world: None,
            options: RenderOptions::default(),
            maps: Arc::new(MapCache::default()),
            cache: HashMap::new(),
            touched: HashMap::new(),
            tick: 0,
            visible: Vec::new(),
            appear: HashMap::new(),
            model: Rc::new(VecModel::default()),
            range: None,
            queue: Vec::new(),
            in_flight: HashSet::new(),
            failed: HashSet::new(),
            debounce: None,
            seq: 0,
        }
    }
}

/// Wires the map's global. Called from `main`, after `content::setup` — the map
/// lives in the saves panel, but its state is its own.
pub fn setup(ui: &App) {
    let weak = ui.as_weak();

    {
        let weak = weak.clone();
        ui.global::<WorldMapState>().on_open(move || {
            let Some(ui) = weak.upgrade() else { return };
            open(&ui, ui.global::<WorldMapState>().get_world());
        });
    }

    {
        let weak = weak.clone();
        ui.global::<WorldMapState>()
            .on_request_tiles(move |x0, x1, z0, z1| {
                let Some(ui) = weak.upgrade() else { return };
                request_tiles(&ui, x0, x1, z0, z1);
            });
    }

    // The model the component draws is this session's own, handed over once:
    // every later write is into the same `VecModel` (see `publish`), so a tile
    // keeps its element for as long as it is on screen.
    let model = STATE.with(|map| Rc::clone(&map.borrow().model));
    ui.global::<WorldMapState>().set_tiles(ModelRc::from(model));
}

/// The component reported which world it is showing. A different one drops
/// every tile: the cache, the model, the queue and the in-flight renders, which
/// is the Vue's `resetWorld` (whose `requestSeq++` is `seq` here).
fn open(ui: &App, source: WorldSource) {
    let world = WorldKey {
        instance_id: source.instance_id.to_string(),
        folder: source.folder.to_string(),
        dimension: if source.dimension.is_empty() {
            "minecraft:overworld".to_string()
        } else {
            source.dimension.to_string()
        },
        tile_size: source.tile_size.max(1) as u32,
    };
    let options = RenderOptions {
        water: source.water,
        shading: source.shading,
        altitude_shading: source.altitude_shading,
    };

    STATE.with(|map| {
        let mut map = map.borrow_mut();
        // The component reports the world from `init` and from every change to
        // it, so this is called repeatedly with the same tuple; only a real
        // change resets anything.
        if map.world.as_ref() == Some(&world) && map.options == options {
            return;
        }
        if let Some(handle) = map.debounce.take() {
            handle.abort();
        }
        map.world = Some(world);
        map.options = options;
        map.cache.clear();
        map.touched.clear();
        map.visible.clear();
        map.appear.clear();
        map.queue.clear();
        map.in_flight.clear();
        map.failed.clear();
        map.range = None;
        map.seq += 1;
    });

    let state = ui.global::<WorldMapState>();
    state.set_error(SharedString::new());
    state.set_fade_until(0.0);
    publish(ui);
}

/// The component asked for a tile range. Everything the Vue did across
/// `dispatchCacheHits`, `pruneRenderCache` and `dispatchTileLoads` happens here,
/// in that order.
fn request_tiles(ui: &App, x0: i32, x1: i32, z0: i32, z1: i32) {
    let range = Range {
        x0: x0.min(x1),
        x1: x0.max(x1),
        z0: z0.min(z1),
        z1: z0.max(z1),
    };

    let changed = STATE.with(|map| {
        let mut map = map.borrow_mut();
        let Some(world) = map.world.clone() else {
            return false;
        };
        if map.range == Some(range) {
            return false;
        }
        map.range = Some(range);

        // Mark everything in the range as freshly wanted, which is what the
        // eviction order reads. Done before anything else so a tile that is
        // about to be re-added to the model is already the newest thing in the
        // cache.
        map.tick = map.tick.wrapping_add(1);
        let tick = map.tick;
        for key in range.tiles() {
            if map.cache.contains_key(&key) {
                map.touched.insert(key, tick);
            }
        }

        // Three lookups, built once, and *after* the queue is trimmed below so a
        // job that was just dropped does not go on blocking its tile. The Vue's
        // three sets were `Set`s and `Map`s, so membership was a hash there
        // too; doing it by scanning a `Vec` instead would make a range change
        // quadratic in the tiles on screen, and a range change is every frame of
        // a drag.
        let on_screen: HashSet<TileKey> = map.visible.iter().copied().collect();

        // The cached half of the model, straight away — the Vue's
        // `dispatchCacheHits`, which ran outside the timer so a pan back over
        // ground it already had did not wait for it.
        //
        // A tile that has left the range leaves the model too, which is what
        // frees the row its element lived in. The Vue's model was implicit — it
        // only ever drew the visible range — so it had no equivalent; the
        // `renderCache` prune is this, one step further out.
        map.visible.retain(|key| range.holds(*key));

        // A tile that is on screen, is cached and is not in the model yet joins
        // it. The order they arrived in is kept rather than re-sorted, so a row
        // holds the tile it was created for and only a genuinely new tile gets a
        // new one.
        let mut fresh: Vec<TileKey> = range
            .tiles()
            .filter(|key| {
                map.cache.contains_key(key)
                    && !map.visible.contains(key)
                    && !on_screen.contains(key)
            })
            .collect();
        fresh.sort_unstable();
        map.visible.extend(fresh);

        // Whatever is queued for a tile that has left the range by more than
        // `QUEUE_MARGIN` is not going to be drawn, and leaving it there is what
        // makes a long pan slow to come back from: the slots go to tiles the
        // user has already panned past. The Vue had the same queue, and got away
        // with it because its cache made the return trip free.
        let wanted = Range {
            x0: range.x0 - QUEUE_MARGIN,
            x1: range.x1 + QUEUE_MARGIN,
            z0: range.z0 - QUEUE_MARGIN,
            z1: range.z1 + QUEUE_MARGIN,
        };
        map.queue.retain(|job| wanted.holds(job.key));

        let busy: HashSet<TileKey> = map
            .queue
            .iter()
            .map(|job| job.key)
            .chain(map.in_flight.iter().copied())
            .collect();

        // The uncached half, queued. The debounce decides when to spend a slot
        // on any of it.
        let tile_size = world.tile_size as i32;
        let seq = map.seq;
        for key in range.tiles() {
            if map.cache.contains_key(&key) || busy.contains(&key) || map.failed.contains(&key) {
                continue;
            }
            map.queue.push(Job {
                key,
                center_x: key.0 * tile_size + tile_size / 2,
                center_z: key.1 * tile_size + tile_size / 2,
                seq,
            });
        }

        if let Some(handle) = map.debounce.take() {
            handle.abort();
        }
        let weak = ui.as_weak();
        map.debounce = Some(crate::runtime::spawn(async move {
            tokio::time::sleep(LOAD_DEBOUNCE).await;
            // `upgrade_in_event_loop` and not `upgrade()`: a Slint handle's
            // strong count lives in the thread that created it, so a weak
            // upgraded from a runtime thread comes back `None`. Every other
            // crossing in this file is the same.
            let _ = weak.upgrade_in_event_loop(|ui| pump(&ui));
        }));
        true
    });

    if changed {
        publish(ui);
    }
}

/// Lets go of cached tiles once the cache is over its cap, oldest first.
///
/// The order is the point. The first version took the first `excess` keys a
/// `HashMap` happened to yield that were not on screen, which is no order at
/// all — and since the tiles behind you are exactly the ones not on screen, a
/// pan evicted the area it had just come from, so panning back re-rendered it.
/// That is the whole of "the cache does not seem to work": the tiles that had
/// been asked for most recently were the likeliest to be the ones dropped.
///
/// [`MapState::touched`] is a counter rather than a clock so the answer does not
/// depend on two tiles having been visible for different lengths of time — it is
/// "which range was asked for last", which is what decides what the user is
/// looking at now and what they are about to pan back to.
fn prune_cache(map: &mut MapState) {
    if map.cache.len() <= MAX_CACHED_TILES {
        return;
    }
    let excess = map.cache.len() - MAX_CACHED_TILES;
    let mut doomed: Vec<(u32, TileKey)> = map
        .cache
        .keys()
        .copied()
        .filter(|key| !map.visible.contains(key))
        .map(|key| (map.touched.get(&key).copied().unwrap_or(0), key))
        .collect();
    doomed.sort_unstable();
    for (_, key) in doomed.into_iter().take(excess) {
        map.cache.remove(&key);
        map.touched.remove(&key);
    }
}

/// Hands the UI the model, index-aligned with `visible` so a row keeps its
/// element — and its GPU-side texture — for as long as its tile is on screen.
fn publish(ui: &App) {
    let now = ui.global::<ScrollInput>().invoke_now_ms();
    STATE.with(|map| {
        let mut map = map.borrow_mut();
        let visible = std::mem::take(&mut map.visible);
        let mut rows = Vec::with_capacity(visible.len());
        for key in &visible {
            let Some(image) = map.cache.get(key).cloned() else {
                continue;
            };
            // A tile is stamped the *first* time it is drawn and never again, so
            // a pan back over cached ground does not flash — the Vue's
            // `tileAppear`, which only `requestTile` ever set.
            let stamp = *map.appear.entry(*key).or_insert(now);
            rows.push(WorldTile {
                tx: key.0,
                tz: key.1,
                image,
                appear_ms: stamp,
            });
        }
        map.visible = visible;
        sync_rows(&map.model, &rows);
    });
}

/// Rewrites a model in place: the rows that are still there are updated, the
/// new ones appended, the gone ones dropped from the end. `content.rs` does the
/// same with `set_row_data`, and for the same reason — a `set_vec` would
/// destroy and re-create every element.
fn sync_rows(model: &VecModel<WorldTile>, rows: &[WorldTile]) {
    let have = model.row_count();
    for (index, row) in rows.iter().enumerate().take(have) {
        if model.row_data(index).as_ref() != Some(row) {
            model.set_row_data(index, row.clone());
        }
    }
    for row in rows.iter().skip(have) {
        model.push(row.clone());
    }
    for index in (rows.len()..have).rev() {
        model.remove(index);
    }
}

/// Which queued tile is closest to the middle of the visible range — the Vue's
/// `pumpRequests` scan, which re-measured every pending tile on every free slot
/// and took the best. Squared distance is enough: it is the same order.
fn nearest_job(queue: &[Job], centre: (f64, f64), tile_size: i32) -> Option<usize> {
    let (cx, cz) = centre;
    (0..queue.len()).min_by_key(|&i| {
        let (tx, tz) = queue[i].key;
        let dx = f64::from(tx * tile_size + tile_size / 2) - cx;
        let dz = f64::from(tz * tile_size + tile_size / 2) - cz;
        (dx * dx + dz * dz) as i64
    })
}

/// Spends the free slots on the queue, nearest tile to the middle of the range
/// first — the Vue's `pumpRequests`, down to its distance metric (a tile's
/// centre against the centre of the visible rectangle).
fn pump(ui: &App) {
    let started = STATE.with(|map| {
        let mut map = map.borrow_mut();
        let Some(world) = map.world.clone() else {
            return Vec::new();
        };
        let Some(range) = map.range else {
            return Vec::new();
        };
        let tile_size = world.tile_size as i32;
        let centre = range.centre(tile_size);

        let mut jobs = Vec::new();
        while map.in_flight.len() < MAX_CONCURRENT {
            let Some(nearest) = nearest_job(&map.queue, centre, tile_size) else {
                break;
            };
            let job = map.queue.remove(nearest);
            map.in_flight.insert(job.key);
            jobs.push(job);
        }
        jobs.into_iter()
            .map(|job| (world.clone(), job))
            .collect::<Vec<_>>()
    });

    for (world, job) in started {
        let maps = STATE.with(|map| Arc::clone(&map.borrow().maps));
        let options = STATE.with(|map| map.borrow().options);
        let weak = ui.as_weak();
        let key = job.key;
        crate::runtime::spawn_blocking(move || {
            let request = WorldMapRequest {
                instance_id: world.instance_id.clone(),
                folder_name: world.folder.clone(),
                width: world.tile_size,
                height: world.tile_size,
                center_x: Some(job.center_x),
                center_z: Some(job.center_z),
                dimension: Some(world.dimension.clone()),
                water: Some(options.water),
                shading: Some(options.shading),
                altitude_shading: Some(options.altitude_shading),
            };
            let result = content::worldmap::render_map(&maps, &request);
            // The render crossed a thread, so what comes back is the buffer
            // rather than the `Image` — the `PendingImage` trick `content.rs`
            // uses for its icons, and for the same reason.
            if weak
                .upgrade_in_event_loop(move |ui| finish_tile(&ui, key, job.seq, result))
                .is_err()
            {
                log::debug!(target: "content", "world map: the window went away mid-render");
            }
        });
    }
}

/// A tile finished rendering, or failed to.
fn finish_tile(
    ui: &App,
    key: TileKey,
    seq: u64,
    result: Result<content::worldmap::WorldMapResult, content::error::Error>,
) {
    let state = ui.global::<WorldMapState>();
    let mut fresh = false;
    let stale = STATE.with(|map| {
        let mut map = map.borrow_mut();
        if map.seq != seq {
            // A different world is open; this render is stale.
            return true;
        }
        map.in_flight.remove(&key);
        match result {
            Ok(result) => {
                let image = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
                    &result.pixels,
                    result.width as u32,
                    result.height as u32,
                ));
                fresh = !map.appear.contains_key(&key);
                map.cache.insert(key, image);
                // A tile that has just been rendered is the newest thing in the
                // cache, so it is the last one the eviction should reach for.
                let tick = map.tick;
                map.touched.insert(key, tick);
                // A tile that arrived is in the range by construction — it was
                // queued from one — so it joins the model now rather than
                // waiting for the next pan. The Vue's `draw()` picked it up on
                // the next frame, which is the same thing here.
                if map.range.is_some_and(|range| range.holds(key)) && !map.visible.contains(&key) {
                    map.visible.push(key);
                }
                prune_cache(&mut map);
                // The Vue clears the error on the first tile that arrives.
                state.set_error(SharedString::new());
            }
            Err(error) => {
                map.failed.insert(key);
                // The Vue only *showed* an error when nothing had loaded at all
                // (`if (tilesLoaded.value === 0)`): one unreadable tile in a
                // world of a hundred is not worth a message over the map.
                if map.cache.is_empty() {
                    state.set_error(SharedString::from(error.to_string()));
                }
            }
        }
        false
    });
    if stale {
        return;
    }

    if fresh {
        // Open the component's fade window. It stops when the clock passes this,
        // and a tile that arrives while it is already open pushes it further
        // out, so a burst of tiles all fade rather than one of them cutting the
        // others short.
        let now = ui.global::<ScrollInput>().invoke_now_ms();
        state.set_fade_until(state.get_fade_until().max(now + FADE_MS));
    }
    publish(ui);

    // A finished render is a free slot, so the next queued tile can start — the
    // Vue's `finally { pumpRequests(requestSeq) }`.
    pump(ui);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(key: TileKey) -> Job {
        Job {
            key,
            center_x: 0,
            center_z: 0,
            seq: 0,
        }
    }

    #[test]
    fn a_range_walks_its_tiles_row_by_row() {
        let range = Range {
            x0: -1,
            x1: 1,
            z0: 4,
            z1: 5,
        };
        assert_eq!(
            range.tiles().collect::<Vec<_>>(),
            vec![(-1, 4), (0, 4), (1, 4), (-1, 5), (0, 5), (1, 5),]
        );
        // A range of one tile is one tile, which is what a map zoomed all the
        // way in asks for.
        let single = Range {
            x0: 3,
            x1: 3,
            z0: 9,
            z1: 9,
        };
        assert_eq!(single.tiles().collect::<Vec<_>>(), vec![(3, 9)]);
    }

    #[test]
    fn the_centre_is_the_middle_of_the_range_in_blocks() {
        // Tiles 0..=2 at 64 blocks each: the middle tile is 1, and its centre is
        // block 96.
        let range = Range {
            x0: 0,
            x1: 2,
            z0: 0,
            z1: 2,
        };
        assert_eq!(range.centre(64), (96.0, 96.0));
        // An even number of tiles has its centre on the seam between the two
        // middle ones, which is what `visibleWorldRect`'s arithmetic gives.
        let even = Range {
            x0: 0,
            x1: 3,
            z0: 0,
            z1: 0,
        };
        assert_eq!(even.centre(64), (128.0, 32.0));
    }

    #[test]
    fn a_range_holds_only_its_own_tiles() {
        let range = Range {
            x0: 0,
            x1: 1,
            z0: 0,
            z1: 1,
        };
        assert!(range.holds((0, 0)));
        assert!(range.holds((1, 1)));
        assert!(!range.holds((2, 1)));
        assert!(!range.holds((-1, 0)));
    }

    #[test]
    fn the_queue_serves_the_middle_of_the_view_first() {
        let tile_size = 64;
        // A 3×3 range centred on tile (1, 1), which is the block (96, 96).
        let centre = Range {
            x0: 0,
            x1: 2,
            z0: 0,
            z1: 2,
        }
        .centre(tile_size);
        let queue = vec![job((0, 0)), job((2, 2)), job((1, 1)), job((0, 2))];
        assert_eq!(nearest_job(&queue, centre, tile_size), Some(2));

        // A wide view's middle tile, queued behind both of its corners.
        let wide = Range {
            x0: 0,
            x1: 4,
            z0: 0,
            z1: 0,
        }
        .centre(tile_size);
        let queue = vec![job((4, 0)), job((0, 0)), job((2, 0))];
        assert_eq!(nearest_job(&queue, wide, tile_size), Some(2));

        // Two tiles equidistant from the middle of a two-wide range, where the
        // scan keeps the one it saw first — the Vue's `if (d < bestDist)`.
        let tie = Range {
            x0: 0,
            x1: 1,
            z0: 0,
            z1: 0,
        }
        .centre(tile_size);
        assert_eq!(
            nearest_job(&[job((0, 0)), job((1, 0))], tie, tile_size),
            Some(0)
        );
        assert_eq!(
            nearest_job(&[job((1, 0)), job((0, 0))], tie, tile_size),
            Some(0)
        );

        assert_eq!(nearest_job(&[], centre, tile_size), None);
    }

    /// A cache over its cap, with a tile the user is looking at marked as the
    /// oldest. `prune_cache` must not touch it.
    fn over_cap_cache() -> MapState {
        let mut map = MapState::new();
        for index in 0..MAX_CACHED_TILES {
            map.cache.insert((index as i32, 0), Image::default());
            map.touched.insert((index as i32, 0), index as u32 + 1);
        }
        map.tick = MAX_CACHED_TILES as u32;
        map
    }

    #[test]
    fn eviction_leaves_the_newest_tiles_and_the_ones_on_screen() {
        let mut map = over_cap_cache();
        // The tile the user is looking at is the oldest thing in the cache.
        let on_screen = (0, 0);
        map.visible.push(on_screen);
        // And a tile that was just asked for is the newest — one the loop above
        // did not already put there, or the cache would not be over its cap.
        let freshest = (100_000, 0);
        map.touched.insert(freshest, map.tick);
        map.cache.insert(freshest, Image::default());

        prune_cache(&mut map);

        assert!(
            map.cache.contains_key(&on_screen),
            "the tile on screen was evicted"
        );
        assert!(
            map.cache.contains_key(&freshest),
            "the tile just asked for was evicted"
        );
        assert_eq!(map.cache.len(), MAX_CACHED_TILES);
        // The oldest off-screen tiles are the ones that went, and their
        // bookkeeping went with them.
        assert!(!map.cache.contains_key(&(1, 0)));
        assert!(!map.touched.contains_key(&(1, 0)));
        assert_eq!(map.touched.len(), map.cache.len());
    }

    #[test]
    fn eviction_is_in_visit_order_not_hash_order() {
        // The bug this covers: the tiles behind the user are exactly the ones
        // not on screen, so evicting "some off-screen tile" evicts where they
        // just came from and a pan back re-renders it. Evicting by visit order
        // instead means the ground panned over most recently survives.
        let mut map = over_cap_cache();
        // The whole cache is stale except the last stretch, which was visited
        // last: stand in for "panned east, came back west".
        for index in 0..MAX_CACHED_TILES as i32 {
            map.touched.insert((index, 0), 1);
        }
        for index in MAX_CACHED_TILES as i32..MAX_CACHED_TILES as i32 + 10 {
            map.cache.insert((index, 0), Image::default());
            map.touched.insert((index, 0), 2);
        }

        prune_cache(&mut map);

        for index in MAX_CACHED_TILES as i32..MAX_CACHED_TILES as i32 + 10 {
            assert!(
                map.cache.contains_key(&(index, 0)),
                "tile {index} was evicted"
            );
        }
        assert!(!map.cache.contains_key(&(0, 0)), "the oldest tile survived");
    }

    #[test]
    fn a_settled_cache_is_left_alone() {
        let mut map = over_cap_cache();
        let before = map.cache.len();
        prune_cache(&mut map);
        assert_eq!(map.cache.len(), before);
    }
}
