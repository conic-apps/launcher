// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{HashMap, hash_map::Entry};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use conic_worldmap::{RenderOptions, RenderRequest, WorldMap};
use storage::LOCATIONS;
use tilemap::{TileImage, TileRequest, TileSource, WorldId};

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
        LOCATIONS
            .instances
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
/// The rectangle is centered on `(center_x, center_z)`; when either coordinate
/// is omitted it falls back to the world spawn. `dimension` is a namespaced id
/// such as `minecraft:the_nether` and defaults to the overworld.
#[derive(Debug, Clone)]
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

/// Render result: an RGBA bitmap, one Minecraft block per pixel, row-major.
#[derive(Debug)]
pub struct WorldMapResult {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

/// Renders a rectangle of a world save into an RGBA bitmap.
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

    Ok(WorldMapResult {
        width: result.width,
        height: result.height,
        pixels: result.pixels,
    })
}

/// The save-backed [`tilemap::TileSource`].
///
/// Holds the [`MapCache`] so the worlds it opens stay warm across tiles. A seed
/// map is a separate source implementing the same port, not a branch here.
#[derive(Default)]
pub struct LocalSaveSource {
    cache: MapCache,
}

impl TileSource for LocalSaveSource {
    fn render(&self, request: &TileRequest) -> std::result::Result<TileImage, tilemap::Error> {
        let WorldId::Save {
            instance_id,
            folder,
            dimension,
        } = &request.world
        else {
            return Err(tilemap::Error(
                "the save source cannot render a seed world".to_string(),
            ));
        };
        let result = render_map(
            &self.cache,
            &WorldMapRequest {
                instance_id: instance_id.clone(),
                folder_name: folder.clone(),
                width: request.size,
                height: request.size,
                center_x: Some(request.center.0),
                center_z: Some(request.center.1),
                dimension: Some(dimension.clone()),
                water: Some(request.options.water),
                shading: Some(request.options.shading),
                altitude_shading: Some(request.options.altitude_shading),
            },
        )
        .map_err(|error| tilemap::Error(error.to_string()))?;
        Ok(TileImage {
            width: result.width as u32,
            height: result.height as u32,
            pixels: result.pixels,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;

    /// A world directory the renderer will open: a `level.dat` naming a data
    /// version the colour tables cover, and the `region/` the legacy overworld
    /// layout keeps its `r.*.*.mca` files in. Empty on the inside, so a tile
    /// comes back transparent — which is all this is about: the request goes
    /// out, an RGBA buffer comes back, and the same world is served from the
    /// cache the second time.
    fn write_world(folder: &str) -> String {
        let world_dir = LOCATIONS
            .instances
            .get_instance_root("worldmap-test")
            .join("saves")
            .join(folder);
        let _ = std::fs::remove_dir_all(&world_dir);
        std::fs::create_dir_all(world_dir.join("region")).expect("world region directory");

        let root: HashMap<String, fastnbt::Value> = HashMap::from([(
            "Data".to_string(),
            fastnbt::Value::Compound(HashMap::from([
                ("DataVersion".to_string(), fastnbt::Value::Int(3700)),
                (
                    "LevelName".to_string(),
                    fastnbt::Value::String(folder.to_string()),
                ),
                ("SpawnX".to_string(), fastnbt::Value::Int(8)),
                ("SpawnZ".to_string(), fastnbt::Value::Int(-24)),
            ])),
        )]);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder
            .write_all(&fastnbt::to_bytes(&root).expect("level.dat"))
            .expect("level.dat");
        std::fs::write(
            world_dir.join("level.dat"),
            encoder.finish().expect("level.dat"),
        )
        .expect("level.dat");
        folder.to_string()
    }

    fn request(folder: &str) -> WorldMapRequest {
        WorldMapRequest {
            instance_id: "worldmap-test".to_string(),
            folder_name: folder.to_string(),
            width: 64,
            height: 64,
            // No centre, so the render falls back to the world's own spawn —
            // the `SpawnX` / `SpawnZ` above.
            center_x: None,
            center_z: None,
            dimension: None,
            water: None,
            shading: None,
            altitude_shading: None,
        }
    }

    #[test]
    fn renders_a_raw_rgba_tile_and_keeps_the_world_cached() {
        let folder = write_world("flatworld");
        let cache = MapCache::default();

        let first = render_map(&cache, &request(&folder)).expect("a tile");
        assert_eq!((first.width, first.height), (64, 64));
        // One block per pixel, four bytes of RGBA — the buffer the Slint side
        // wraps in a `SharedPixelBuffer`. No PNG, no base64.
        assert_eq!(first.pixels.len(), 64 * 64 * 4);

        // A second tile of the same world is served with the world still open,
        // which is what `MapCache` is for: the region cache inside
        // `conic-worldmap` is not rebuilt from the disk.
        let second = render_map(&cache, &request(&folder)).expect("a second tile");
        assert_eq!(second.pixels.len(), 64 * 64 * 4);
        assert_eq!(cache.maps.lock().expect("Internal error").len(), 1);
    }

    #[test]
    fn a_centred_tile_comes_from_the_requested_rectangle() {
        let folder = write_world("centred");
        let cache = MapCache::default();
        let mut request = request(&folder);
        request.center_x = Some(1000);
        request.center_z = Some(-1000);

        let tile = render_map(&cache, &request).expect("a tile");
        assert_eq!((tile.width, tile.height), (64, 64));
    }

    #[test]
    fn a_missing_world_is_an_error_rather_than_an_empty_tile() {
        let cache = MapCache::default();
        let mut request = request("no-such-world");
        request.dimension = Some("minecraft:the_nether".to_string());
        assert!(render_map(&cache, &request).is_err());
    }
}
