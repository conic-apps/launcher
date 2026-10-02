// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world map's open callbacks, registered on `WorldMapState`.

use super::*;

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
/// every tile: the cache, the model, the queue and the in-flight renders, and
/// bumps `seq`.
pub(crate) fn open(ui: &App, source: WorldSource) {
    let world = WorldId::Save {
        instance_id: source.instance_id.to_string(),
        folder: source.folder.to_string(),
        dimension: if source.dimension.is_empty() {
            "minecraft:overworld".to_string()
        } else {
            source.dimension.to_string()
        },
    };
    let tile_size = source.tile_size.max(1) as u32;
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
        if map.world.as_ref() == Some(&world)
            && map.tile_size == tile_size
            && map.options == options
        {
            return;
        }
        if let Some(handle) = map.debounce.take() {
            handle.abort();
        }
        map.source = Some(crate::usecases::worldmap::source_for(&world));
        map.world = Some(world);
        map.tile_size = tile_size;
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
