// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world map's "script": the tile cache and the render queue behind
//! `ui/components/world-map.slint`.
//!
//! # The tile cache
//!
//! A Slint `Image` is a reference-counted handle over a `SharedPixelBuffer`:
//! cloning one is free, there is no encoded form to keep a second copy of, and
//! the renderer drops its own GPU copy under memory pressure on its own. So
//! there is **one** cache, here, keyed by tile and holding the finished `Image`:
//! a tile is either cached or not yet rendered, with no second tier to keep in
//! step. The model the component draws is a *view* of it — the tiles in the
//! visible range — and it is written row by row, so a tile keeps its element for
//! as long as it is on screen.
//!
//! # The load queue
//!
//! A pending set, an in-flight set, a nearest-to-centre priority scan and a 50ms
//! debounce, all driven from the event loop. The model is refilled from the cache
//! **immediately** when the range moves, and only the *renders* are debounced.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use slint::{ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel};

use crate::slint_backend::{App, ScrollInput, WorldMapState, WorldSource, WorldTile};
use content::worldmap::{MapCache, WorldMapRequest};

/// How many tiles may be rendering at once.
///
/// A render walks every block column in a 64×64 tile, so a few dozen in flight
/// saturate the disk and the CPU and everything past that is only queueing.
pub(crate) const MAX_CONCURRENT: usize = 24;

/// A drag or a zoom glide moves the visible range many times a second, and every
/// move would otherwise queue a fresh batch of renders for a range the user is
/// about to leave.
pub(crate) const LOAD_DEBOUNCE: Duration = Duration::from_millis(50);

/// The component's own fade clock runs until this much time has passed; Rust
/// only needs it to publish how long the fade window stays open.
pub(crate) const FADE_MS: f32 = 100.0;

/// How many tiles the cache holds before it starts letting go.
///
/// A 64×64 RGBA tile is 16 KiB, and a map the user has panned right across is
/// thousands of them — 2048 is about 32 MiB, and roughly twenty screens of the
/// default zoom, which is what makes coming back to where you were a hit rather
/// than a re-render.
///
/// What is dropped is re-rendered on demand, off the region cache the world
/// keeps alive in `MapCache`, which is what a cache miss costs.
pub(crate) const MAX_CACHED_TILES: usize = 2048;

/// How far outside the visible range a queued render is kept.
///
/// A drag or a zoom glide moves the range many times a second, and without this
/// the queue would keep every tile of every range it passed through: a long pan
/// leaves hundreds of jobs for tiles that are off screen, all of them competing
/// for the 24 slots with the ones the user is actually looking at. A tile that
/// leaves by more than this is very unlikely to be wanted again, and dropping it
/// from the queue costs nothing — the cache is what remembers it.
pub(crate) const QUEUE_MARGIN: i32 = 8;

/// A tile's position in the tile grid: which square of the world it is. This is
/// the key of the cache, of the model and of the queue, and it is what the
/// component multiplies by the tile size to place a tile.
pub(crate) type TileKey = (i32, i32);

/// The world a tile belongs to, and how it is rendered. The cache's own key: a
/// different save is a different map and its tiles mean nothing here. This is
/// the `WorldSource` the component reports, reduced to what a cache key needs.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WorldKey {
    instance_id: String,
    folder: String,
    dimension: String,
    tile_size: u32,
}

/// What one tile of the queue needs to be rendered.
#[derive(Clone)]
pub(crate) struct Job {
    key: TileKey,
    /// The world block at the tile's centre — a render request is *centred*
    /// (`RenderRequest::new`).
    center_x: i32,
    center_z: i32,
    /// Which world the job belongs to. Checked on arrival, so a render that
    /// lands after the user has picked a different save is dropped.
    seq: u64,
}

pub(crate) struct MapState {
    /// The world on screen. `None` until a map reports one.
    world: Option<WorldKey>,
    /// The render options the component declared, which every tile is asked for.
    options: RenderOptions,
    /// The `conic-worldmap` worlds, kept alive so a tile that scrolls back into
    /// view re-reads a warm region cache instead of the disk — `content`'s
    /// `MapCache`.
    maps: Arc<MapCache>,
    /// Every tile rendered for `world`, by position: there is nothing cheaper to
    /// keep than the finished image, and nothing to decode.
    cache: HashMap<TileKey, Image>,
    /// When each cached tile was last inside a visible range, as a counter off
    /// [`MapState::tick`]. The eviction order — see [`prune_cache`].
    touched: HashMap<TileKey, u32>,
    tick: u32,
    /// The tiles in the model, in the order they were added. A tile leaves when
    /// it is dropped from here, and the model's rows are index-aligned with it.
    visible: Vec<TileKey>,
    /// When each tile first reached the model, for the fade. Kept after the tile
    /// has faded, so a pan back over cached ground does not flash.
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
    /// Tiles a render failed on, so a region that cannot be read is not asked
    /// for again.
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
pub(crate) struct RenderOptions {
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
pub(crate) struct Range {
    x0: i32,
    x1: i32,
    z0: i32,
    z1: i32,
}

impl Range {
    /// Every tile of the range, row by row.
    fn tiles(&self) -> impl Iterator<Item = TileKey> + '_ {
        (self.z0..=self.z1).flat_map(|tz| (self.x0..=self.x1).map(move |tx| (tx, tz)))
    }

    /// The middle of the range in world blocks, which is what the queue measures
    /// a tile's distance from.
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

mod tiles;
mod wiring;

pub(crate) use tiles::*;
pub(crate) use wiring::*;

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
        // middle ones.
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

        // Two tiles equidistant from the middle of a two-wide range: the scan
        // keeps the one it saw first.
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
