// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The tile queue: what to load, how to load it and where to draw it.

use super::*;

/// The component asked for a tile range. Everything the Vue did across
/// `dispatchCacheHits`, `pruneRenderCache` and `dispatchTileLoads` happens here,
/// in that order.
pub(crate) fn request_tiles(ui: &App, x0: i32, x1: i32, z0: i32, z1: i32) {
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
pub(crate) fn prune_cache(map: &mut MapState) {
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
pub(crate) fn publish(ui: &App) {
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
pub(crate) fn sync_rows(model: &VecModel<WorldTile>, rows: &[WorldTile]) {
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
pub(crate) fn nearest_job(queue: &[Job], centre: (f64, f64), tile_size: i32) -> Option<usize> {
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
pub(crate) fn pump(ui: &App) {
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
pub(crate) fn finish_tile(
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
