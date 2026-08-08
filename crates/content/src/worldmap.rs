// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{HashMap, hash_map::Entry};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use base64::{Engine, engine::general_purpose};
use conic_worldmap::{RenderOptions, RenderRequest, WorldMap};
use folder::DATA_LOCATION;
use serde::{Deserialize, Serialize};
use tauri::{Manager, Runtime, command};

use crate::error::*;

const OVERWORLD_DIMENSION: &str = "minecraft:overworld";
const MAX_CACHED_WORLDS: usize = 8;

/// Managed state: keeps open worlds alive so repeated renders reuse the
/// internal chunk cache of conic-worldmap. Worlds are shared through `Arc` so
/// the map lock is only held to look up/open a world, letting tiles of the
/// same world render concurrently.
#[derive(Default)]
pub struct MapCache {
    maps: Mutex<HashMap<WorldMapKey, Arc<WorldMap>>>,
}

/// Identifies an open world inside the cache.
#[derive(Clone, PartialEq, Eq, Hash)]
struct WorldMapKey {
    instance_id: String,
    folder_name: String,
    dimension: String,
}

impl WorldMapKey {
    fn world_dir(&self) -> PathBuf {
        DATA_LOCATION
            .get_instance_root(&self.instance_id)
            .join("saves")
            .join(&self.folder_name)
    }
}

impl From<&WorldMapRequest> for WorldMapKey {
    fn from(request: &WorldMapRequest) -> Self {
        WorldMapKey {
            instance_id: request.instance_id.clone(),
            folder_name: request.folder_name.clone(),
            dimension: request
                .dimension
                .clone()
                .unwrap_or_else(|| OVERWORLD_DIMENSION.to_string()),
        }
    }
}

/// A map render request: an axis-aligned rectangle of world blocks.
///
/// The rectangle is centered on `(center_x, center_z)`; when the center is
/// omitted it falls back to the world spawn. `dimension` is a namespaced id
/// such as `minecraft:the_nether` and defaults to the overworld.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldMapRequest {
    pub instance_id: String,
    pub folder_name: String,
    pub width: u32,
    pub height: u32,
    pub center_x: Option<i32>,
    pub center_z: Option<i32>,
    pub dimension: Option<String>,
    pub water: Option<bool>,
    pub shading: Option<bool>,
    pub altitude_shading: Option<bool>,
}

/// Render result: a PNG-encoded bitmap (base64-encoded), one Minecraft block
/// per pixel, row-major.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldMapResult {
    pub width: usize,
    pub height: usize,
    pub png: String,
}

/// Renders a rectangle of a world save into a PNG bitmap.
pub fn render_map(cache: &MapCache, request: &WorldMapRequest) -> Result<WorldMapResult> {
    let key = WorldMapKey::from(request);
    let world_dir = key.world_dir();
    let dimension = key.dimension.clone();

    let mut maps = cache.maps.lock().expect("Internal error");
    if !maps.contains_key(&key)
        && maps.len() >= MAX_CACHED_WORLDS
        && let Some(oldest) = maps.keys().next().cloned()
    {
        maps.remove(&oldest);
    }
    let world = match maps.entry(key) {
        Entry::Occupied(entry) => entry.into_mut().clone(),
        Entry::Vacant(entry) => entry
            .insert(Arc::new(WorldMap::open_dimension(world_dir, &dimension)?))
            .clone(),
    };
    drop(maps);

    let (center_x, center_z) = match (request.center_x, request.center_z) {
        (Some(x), Some(z)) => (x, z),
        _ => world.spawn(),
    };

    let result = world.render(
        &RenderRequest::new(center_x, center_z, request.width, request.height),
        &RenderOptions {
            water: request.water.unwrap_or(true),
            shading: request.shading.unwrap_or(true),
            altitude_shading: request.altitude_shading.unwrap_or(true),
        },
    )?;

    let png = encode_png(result.width as u32, result.height as u32, &result.pixels)?;

    Ok(WorldMapResult {
        width: result.width,
        height: result.height,
        png: general_purpose::STANDARD.encode(png),
    })
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| Error::WorldMapPng(e.to_string()))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| Error::WorldMapPng(e.to_string()))?;
    }
    Ok(png)
}

#[command]
pub(crate) async fn cmd_render_world_map<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: WorldMapRequest,
) -> Result<WorldMapResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let cache = app.state::<MapCache>();
        render_map(&cache, &request)
    })
    .await
    .map_err(|e| Error::WorldMapTask(e.to_string()))?
}
