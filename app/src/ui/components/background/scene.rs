// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The 3D background world: terrain, trees and face culling. It emits the face
//! list plus the GPU vertex and index buffers, rebuilt when the camera crosses a
//! block or the aspect changes.

use std::collections::HashSet;

pub const CAMERA_SPEED: f32 = 2.0;
/// The frustum's slope in x per unit z: `tan(60° / 2)`.
pub const FOV_HALF_TAN: f32 = 0.577_350_26;
pub const NEAR_PLANE: f32 = 0.2;
pub const VIEW_DISTANCE: i32 = 120;
pub const FADE_START: f32 = VIEW_DISTANCE as f32 * 0.55;
pub const FADE_END: f32 = VIEW_DISTANCE as f32;

/// x sits between two block columns so the world scrolls past evenly.
pub const CAM_X: f32 = -0.5;
pub const CAM_Y: f32 = BASE_HEIGHT as f32 + EYE_HEIGHT;

pub const LAYER_ALPHA: f32 = 0.3;
/// Expanded into screen-space quads, so this is a width in device pixels rather
/// than in the world.
pub const EDGE_WIDTH_DEVICE_PX: f32 = 2.0;

const BASE_HEIGHT: i32 = 4;
const HILL_AMPLITUDE: f32 = 2.2;
const EYE_HEIGHT: f32 = 10.6;
const TREE_DENSITY: f64 = 0.0007;
const WORLD_STROKE_ALPHA: f32 = 0.55;
const FILL_ALPHA_NEAR: f32 = 1.0;
const FILL_ALPHA_FAR: f32 = 0.15;

/// The block column the camera travels down. A tree whose blocks would occupy
/// it is not grown; with the current camera height none can reach it, but the
/// check would matter if the height changed.
const CAM_X_CELL: i32 = -1;
const CAM_Y_CELL: i32 = 14;

/// Half the widest row of columns the frustum can see, plus a margin.
const MAX_HALF_X: i32 = 74;

/// Keeps the frustum cull conservative past a tree's sideways reach.
const TREE_OVERHANG: f32 = 3.0;

const HEIGHT_X_OFF: i32 = -79;
const HEIGHT_X_SPAN: i32 = 164;
const HEIGHT_Z_SPAN: i32 = 128;

const CORNER_OFFSETS: [[f32; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
    [0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
];

/// Corner order of each visible face. The bottom face is never seen: the camera
/// is above every block.
const FACE_NZ: [usize; 4] = [0, 1, 3, 2];
const FACE_NX: [usize; 4] = [0, 2, 6, 4];
const FACE_PX: [usize; 4] = [1, 5, 7, 3];
const FACE_PY: [usize; 4] = [2, 3, 7, 6];
const FACE_NY: [usize; 4] = [0, 1, 5, 4];

/// Tree shape from the block above the surface: `[dx, dy, dz]`.
const TREE_SHAPE: [[i32; 3]; 68] = [
    // trunk, four blocks tall
    [0, 0, 0],
    [0, 1, 0],
    [0, 2, 0],
    [0, 3, 0],
    // lower leaves (5x5)
    [-2, 4, -1],
    [-1, 4, -1],
    [0, 4, -1],
    [1, 4, -1],
    [2, 4, -1],
    [-2, 4, 0],
    [-1, 4, 0],
    [0, 4, 0],
    [1, 4, 0],
    [2, 4, 0],
    [-2, 4, 1],
    [-1, 4, 1],
    [0, 4, 1],
    [1, 4, 1],
    [2, 4, 1],
    [-1, 4, -2],
    [0, 4, -2],
    [1, 4, -2],
    [-1, 4, 2],
    [0, 4, 2],
    [1, 4, 2],
    [-2, 4, -2],
    [2, 4, -2],
    [-2, 4, 2],
    [2, 4, 2],
    // middle leaves (5x5, with the trunk showing through)
    [2, 3, 2],
    [-2, 3, -2],
    [2, 3, -2],
    [-2, 3, 2],
    [-2, 3, -1],
    [-1, 3, -1],
    [0, 3, -1],
    [1, 3, -1],
    [2, 3, -1],
    [-2, 3, 0],
    [-1, 3, 0],
    [0, 3, 0],
    [1, 3, 0],
    [2, 3, 0],
    [-2, 3, 1],
    [-1, 3, 1],
    [0, 3, 1],
    [1, 3, 1],
    [2, 3, 1],
    [-1, 3, -2],
    [0, 3, -2],
    [1, 3, -2],
    [-1, 3, 2],
    [0, 3, 2],
    [1, 3, 2],
    // upper leaves (3x3, with a cross on top)
    [-1, 5, -1],
    [0, 5, -1],
    [1, 5, -1],
    [-1, 5, 0],
    [0, 5, 0],
    [1, 5, 0],
    [-1, 5, 1],
    [0, 5, 1],
    [1, 5, 1],
    [0, 6, -1],
    [-1, 6, 0],
    [0, 6, 0],
    [1, 6, 0],
    [0, 6, 1],
];

#[derive(Clone, Copy)]
pub struct Face {
    pub corners: [[f32; 3]; 4],
    pub fill_alpha: f32,
    /// How far the outline is mixed toward the edge colour (0 at the fog end).
    pub edge_mix: f32,
}

/// The world's geometry for one camera block.
pub struct Scene {
    /// Ring-buffered column heights; 0 means "not computed" (heights are 2..7).
    heights: Vec<f32>,
    tree_blocks: HashSet<(i32, i32, i32)>,
    /// Faces in emission order, which decides how overlapping fills and tied
    /// edges resolve.
    pub faces: Vec<Face>,
    pub ground: [[f32; 3]; 4],
    /// The GPU path's buffers, built only when a GPU renderer will draw them.
    pub fill_vertices: Vec<f32>,
    pub fill_indices: Vec<u32>,
    pub edge_vertices: Vec<f32>,
    built_floor: Option<i32>,
    built_aspect: f32,
    buffers: bool,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            heights: vec![0.0; (HEIGHT_X_SPAN * HEIGHT_Z_SPAN) as usize],
            tree_blocks: HashSet::new(),
            faces: Vec::new(),
            ground: [[0.0; 3]; 4],
            fill_vertices: Vec::new(),
            fill_indices: Vec::new(),
            edge_vertices: Vec::new(),
            built_floor: None,
            built_aspect: 0.0,
            buffers: false,
        }
    }
}

/// What an [`Scene::update`] changed, so the GPU path can upload only that.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rebuilt {
    Nothing,
    /// The ground slab, whose reach depends on the aspect.
    Ground,
    /// The whole world: the camera crossed a block or the buffer set changed.
    World,
}

impl Scene {
    /// Rebuilds when the camera block, the aspect or the buffer set changed.
    pub fn update(&mut self, cam_z: f32, aspect: f32, buffers: bool) -> Rebuilt {
        let floor = cam_z.floor() as i32;
        // The aspect decides how far down the ground quad reaches. The terrain
        // itself does not depend on it, so an aspect-only change updates four
        // ground vertices instead of rebuilding every face.
        let aspect_changed = (aspect - self.built_aspect).abs() > 1e-4;
        if self.built_floor == Some(floor) && self.buffers == buffers {
            if !aspect_changed {
                return Rebuilt::Nothing;
            }
            self.built_aspect = aspect;
            self.update_ground(cam_z, aspect);
            return Rebuilt::Ground;
        }
        self.built_floor = Some(floor);
        self.built_aspect = aspect;
        self.buffers = buffers;
        self.build(cam_z, aspect);
        Rebuilt::World
    }

    /// The slab's four corners. Its horizontal reach is fixed; only the
    /// vertical reach follows the aspect, because a taller window sees farther
    /// down the far plane.
    fn ground_quad(cam_z: f32, aspect: f32) -> [[f32; 3]; 4] {
        let x_reach = FADE_END * FOV_HALF_TAN;
        let y_reach = FADE_END * aspect * FOV_HALF_TAN;
        let wz = cam_z + FADE_END;
        [
            [CAM_X - x_reach, CAM_Y, wz],
            [CAM_X + x_reach, CAM_Y, wz],
            [CAM_X - x_reach, CAM_Y - y_reach, wz],
            [CAM_X + x_reach, CAM_Y - y_reach, wz],
        ]
    }

    /// Moves the ground slab without touching the terrain. The four vertices
    /// are the first ones `build` writes, followed by their two triangles.
    fn update_ground(&mut self, cam_z: f32, aspect: f32) {
        self.ground = Self::ground_quad(cam_z, aspect);
        if !self.buffers {
            return;
        }
        debug_assert!(self.fill_vertices.len() >= 16);
        for (vertex, corner) in self.fill_vertices[..16]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.ground)
        {
            vertex.copy_from_slice(&[corner[0], corner[1], corner[2], 1.0]);
        }
    }

    fn build(&mut self, cam_z: f32, aspect: f32) {
        let z_base = cam_z.floor() as i32;
        let z_near = z_base + 1;
        let z_far = z_base + VIEW_DISTANCE;
        let x_min = (CAM_X - MAX_HALF_X as f32).floor() as i32;
        let x_max = (CAM_X + MAX_HALF_X as f32).ceil() as i32;

        self.invalidate_height_row(z_far);
        self.tree_blocks.clear();
        self.faces.clear();
        self.fill_vertices.clear();
        self.fill_indices.clear();
        self.edge_vertices.clear();

        // The ground base quad: a background-coloured slab on the far plane
        // that hides the hyperbola sky below the horizon.
        self.ground = Self::ground_quad(cam_z, aspect);
        if self.buffers {
            for corner in self.ground {
                self.fill_vertices
                    .extend_from_slice(&[corner[0], corner[1], corner[2], 1.0]);
            }
            self.fill_indices.extend_from_slice(&[0, 1, 2, 1, 3, 2]);
        }

        // Tree blocks are needed first: the columns and the trees use them for
        // occlusion tests.
        for z in z_base..=z_far {
            for x in x_min..=x_max {
                if !self.column_may_be_visible(x, z, cam_z) {
                    continue;
                }
                if self.tree_at(x, z) {
                    let y0 = self.height_at(x, z) + 1;
                    for [dx, dy, dz] in TREE_SHAPE {
                        self.tree_blocks.insert((x + dx, y0 + dy, z + dz));
                    }
                }
            }
        }

        for z in (z_near..=z_far).rev() {
            for x in x_min..=x_max {
                self.build_column(x, z, cam_z);
            }
        }
    }

    /// Skips columns that cannot project into the viewport, using the block's
    /// nearest camera position because the frustum widens with distance.
    fn column_may_be_visible(&self, x: i32, z: i32, cam_z: f32) -> bool {
        let dz = (z - cam_z.floor() as i32 - 1) as f32;
        let reach = FOV_HALF_TAN * dz.max(0.0) + TREE_OVERHANG;
        (x as f32 + 1.0 + TREE_OVERHANG) >= CAM_X - reach
            && (x as f32 - TREE_OVERHANG) <= CAM_X + reach
    }

    /// A deterministic hash of a lattice point, operation for operation what the
    /// original JavaScript did, so the terrain reproduces the golden table.
    fn hash2i(ix: i32, iz: i32, seed: i32) -> f64 {
        let mut h = ix.wrapping_mul(374_761_393) ^ iz.wrapping_mul(668_265_263);
        h = h.wrapping_add(seed);
        h = (h ^ ((h as u32 >> 13) as i32)).wrapping_mul(1_274_126_177);
        h ^= (h as u32 >> 16) as i32;
        f64::from(h as u32) / 4_294_967_296.0
    }

    fn value_noise(x: f64, z: f64, seed: i32) -> f64 {
        let ix = x.floor();
        let iz = z.floor();
        let fx = x - ix;
        let fz = z - iz;
        let ux = fx * fx * (3.0 - 2.0 * fx);
        let uz = fz * fz * (3.0 - 2.0 * fz);
        let (ix, iz) = (ix as i32, iz as i32);
        let a = Self::hash2i(ix, iz, seed);
        let b = Self::hash2i(ix + 1, iz, seed);
        let c = Self::hash2i(ix, iz + 1, seed);
        let d = Self::hash2i(ix + 1, iz + 1, seed);
        a + (b - a) * ux + (c - a) * uz + (a - b - c + d) * ux * uz
    }

    fn terrain_height(x: i32, z: i32) -> i32 {
        let x = f64::from(x);
        let z = f64::from(z);
        let n1 = (Self::value_noise(x / 14.0, z / 14.0, 0x71) - 0.5) * 2.0;
        let n2 = (Self::value_noise(x / 4.5, z / 4.5, 0x3a) - 0.5) * 1.2;
        let h = (f64::from(BASE_HEIGHT) + n1 * f64::from(HILL_AMPLITUDE) + n2).round() as i32;
        h.clamp(2, 7)
    }

    /// The ring cache avoids regenerating a height for the several queries a
    /// column and its neighbours make.
    fn height_at(&mut self, x: i32, z: i32) -> i32 {
        let xi = x - HEIGHT_X_OFF;
        if !(0..HEIGHT_X_SPAN).contains(&xi) {
            return Self::terrain_height(x, z);
        }
        let zm = z.rem_euclid(HEIGHT_Z_SPAN);
        let index = (xi + zm * HEIGHT_X_SPAN) as usize;
        let mut value = self.heights[index];
        if value == 0.0 {
            value = Self::terrain_height(x, z) as f32;
            self.heights[index] = value;
        }
        value as i32
    }

    fn invalidate_height_row(&mut self, z_world: i32) {
        let zm = z_world.rem_euclid(HEIGHT_Z_SPAN);
        let start = (zm * HEIGHT_X_SPAN) as usize;
        self.heights[start..start + HEIGHT_X_SPAN as usize].fill(0.0);
    }

    fn tree_at(&mut self, x: i32, z: i32) -> bool {
        let h = self.height_at(x, z);
        if !(3..=6).contains(&h) {
            return false;
        }
        if Self::hash2i(x, z, 0x5eed) >= TREE_DENSITY {
            return false;
        }
        !Self::tree_hits_camera(x, h)
    }

    fn tree_hits_camera(x: i32, h: i32) -> bool {
        let y0 = h + 1;
        TREE_SHAPE
            .iter()
            .any(|[dx, dy, _]| x + dx == CAM_X_CELL && y0 + dy == CAM_Y_CELL)
    }

    /// Below the world (`y < 1`) counts as solid, so columns have no underside.
    fn is_solid(&mut self, x: i32, y: i32, z: i32) -> bool {
        if y < 1 {
            return true;
        }
        if y <= self.height_at(x, z) {
            return true;
        }
        self.tree_blocks.contains(&(x, y, z))
    }

    fn emit_face(&mut self, x: i32, y: i32, z: i32, face: &[usize; 4], cam_z: f32) {
        let zf = z as f32;
        if zf - cam_z < 0.05 {
            return;
        }
        let dz = zf + 0.5 - cam_z;
        let fade = ((dz - FADE_START) / (FADE_END - FADE_START)).clamp(0.0, 1.0);
        if fade >= 1.0 {
            return;
        }
        let fill_alpha = FILL_ALPHA_NEAR + (FILL_ALPHA_FAR - FILL_ALPHA_NEAR) * fade;
        let edge_mix = WORLD_STROKE_ALPHA * (1.0 - fade);
        let mut corners = [[0.0f32; 3]; 4];
        for (slot, &corner) in corners.iter_mut().zip(face.iter()) {
            let offset = CORNER_OFFSETS[corner];
            *slot = [x as f32 + offset[0], y as f32 + offset[1], zf + offset[2]];
        }
        if self.buffers {
            let base = (self.fill_vertices.len() / 4) as u32;
            for corner in corners {
                self.fill_vertices
                    .extend_from_slice(&[corner[0], corner[1], corner[2], fill_alpha]);
            }
            self.fill_indices.extend_from_slice(&[
                base,
                base + 1,
                base + 2,
                base,
                base + 2,
                base + 3,
            ]);
            // Two triangles per edge, each vertex naming the other end so the
            // vertex shader can work out the screen-space normal.
            for index in 0..4 {
                let from = corners[index];
                let to = corners[(index + 1) % 4];
                for (first, second, side) in [
                    (from, to, 1.0f32),
                    (from, to, -1.0),
                    (to, from, 1.0),
                    (from, to, -1.0),
                    (to, from, 1.0),
                    (to, from, -1.0),
                ] {
                    self.edge_vertices.extend_from_slice(&[
                        first[0], first[1], first[2], edge_mix, second[0], second[1], second[2],
                        side,
                    ]);
                }
            }
        }
        self.faces.push(Face {
            corners,
            fill_alpha,
            edge_mix,
        });
    }

    fn emit_block(&mut self, x: i32, y: i32, z: i32, cam_z: f32) {
        if !self.is_solid(x, y, z - 1) {
            self.emit_face(x, y, z, &FACE_NZ, cam_z);
        }
        if CAM_X < x as f32 && !self.is_solid(x - 1, y, z) {
            self.emit_face(x, y, z, &FACE_NX, cam_z);
        }
        if CAM_X > (x + 1) as f32 && !self.is_solid(x + 1, y, z) {
            self.emit_face(x, y, z, &FACE_PX, cam_z);
        }
        if CAM_Y > (y + 1) as f32 && !self.is_solid(x, y + 1, z) {
            self.emit_face(x, y, z, &FACE_PY, cam_z);
        }
        if CAM_Y < y as f32 && !self.is_solid(x, y - 1, z) {
            self.emit_face(x, y, z, &FACE_NY, cam_z);
        }
    }

    fn build_column(&mut self, x: i32, z: i32, cam_z: f32) {
        if !self.column_may_be_visible(x, z, cam_z) {
            return;
        }
        let h = self.height_at(x, z);
        let h_front = self.height_at(x, z - 1);

        // Blocks below the lowest of the column and the neighbours it could be
        // seen behind are hidden and skipped.
        let mut side_h = h_front;
        if x >= 1 {
            side_h = side_h.min(self.height_at(x - 1, z));
        }
        if x <= -2 {
            side_h = side_h.min(self.height_at(x + 1, z));
        }
        let start = 1.max(h.min(h_front).min(side_h));
        for y in start..=h {
            self.emit_block(x, y, z, cam_z);
        }

        if self.tree_at(x, z) {
            let y0 = h + 1;
            for [dx, dy, dz] in TREE_SHAPE {
                self.emit_block(x + dx, y0 + dy, z + dz, cam_z);
            }
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values from the original JavaScript implementation.
    const GOLDEN: [(i32, i32, i32, f64); 50] = [
        (-80, 17, 5, 0.768376867),
        (-80, 83, 5, 0.816261004),
        (-73, 6, 4, 0.885144234),
        (-73, 72, 4, 0.983211281),
        (-66, -5, 3, 0.937576341),
        (-66, 61, 4, 0.194039508),
        (-66, 127, 6, 0.651138775),
        (-59, 50, 4, 0.293786458),
        (-59, 116, 4, 0.377017551),
        (-52, 39, 3, 0.552817388),
        (-52, 105, 5, 0.050319094),
        (-45, 28, 3, 0.924709975),
        (-45, 94, 6, 0.707577512),
        (-38, 17, 5, 0.328011554),
        (-38, 83, 5, 0.946821891),
        (-31, 6, 5, 0.693729761),
        (-31, 72, 4, 0.147399206),
        (-24, -5, 3, 0.285186836),
        (-24, 61, 4, 0.372041897),
        (-24, 127, 3, 0.514515708),
        (-17, 50, 3, 0.786898298),
        (-17, 116, 3, 0.875484306),
        (-10, 39, 3, 0.939406444),
        (-10, 105, 5, 0.396027603),
        (-3, 28, 2, 0.154870614),
        (-3, 94, 4, 0.489853877),
        (4, 17, 3, 0.460725674),
        (4, 83, 4, 0.911143562),
        (11, 6, 4, 0.243297265),
        (11, 72, 5, 0.496204261),
        (18, -5, 4, 0.721459016),
        (18, 61, 5, 0.784185876),
        (18, 127, 3, 0.642566028),
        (25, 50, 5, 0.878788158),
        (25, 116, 3, 0.798986070),
        (32, 39, 4, 0.899506445),
        (32, 105, 4, 0.229072270),
        (39, 28, 3, 0.053760217),
        (39, 94, 6, 0.525065606),
        (46, 17, 4, 0.218300324),
        (46, 83, 5, 0.966226280),
        (53, 6, 4, 0.461425038),
        (53, 72, 4, 0.972866307),
        (60, -5, 4, 0.955806376),
        (60, 61, 5, 0.645592182),
        (60, 127, 4, 0.903055802),
        (67, 50, 6, 0.882055077),
        (67, 116, 5, 0.825082180),
        (74, 39, 5, 0.826588593),
        (74, 105, 5, 0.591652714),
    ];

    /// A resize changes the aspect, and the terrain does not depend on it.
    /// Only the ground slab may move; rebuilding the faces would put a full
    /// scene build on every resize frame.
    #[test]
    fn an_aspect_change_only_moves_the_ground() {
        let mut scene = Scene::new();
        assert_eq!(scene.update(0.0, 0.5, true), Rebuilt::World);

        let faces = scene.faces.len();
        let edges = scene.edge_vertices.clone();
        let fill_tail = scene.fill_vertices[16..].to_vec();
        let ground = scene.ground;

        assert_eq!(scene.update(0.0, 0.75, true), Rebuilt::Ground);
        assert_eq!(scene.faces.len(), faces);
        assert_eq!(scene.edge_vertices, edges);
        assert_eq!(&scene.fill_vertices[16..], &fill_tail[..]);
        assert_ne!(scene.ground, ground);
        for (vertex, corner) in scene.fill_vertices[..16]
            .as_chunks::<4>()
            .0
            .iter()
            .zip(scene.ground)
        {
            assert_eq!(&vertex[..3], &corner[..]);
            assert_eq!(vertex[3], 1.0);
        }

        assert_eq!(scene.update(0.0, 0.75, true), Rebuilt::Nothing);
    }

    #[test]
    fn terrain_matches_the_original() {
        for (x, z, height, hash) in GOLDEN {
            assert_eq!(Scene::terrain_height(x, z), height, "height at ({x}, {z})");
            assert!(
                (Scene::hash2i(x, z, 0x5eed) - hash).abs() < 1e-9,
                "hash at ({x}, {z}): {} != {hash}",
                Scene::hash2i(x, z, 0x5eed)
            );
        }
    }

    /// The ring cache must agree with the generator, including across the wrap
    /// where the 128th row reuses the first's slots.
    #[test]
    fn height_cache_matches_the_generator() {
        let mut scene = Scene::new();
        let mut cam_z = 0.0f32;
        while cam_z < 300.0 {
            let base = cam_z.floor() as i32;
            scene.invalidate_height_row(base + VIEW_DISTANCE);
            for z in base..=(base + VIEW_DISTANCE) {
                for x in [-79, -40, 0, 73, 74, 100] {
                    assert_eq!(
                        scene.height_at(x, z),
                        Scene::terrain_height(x, z),
                        "cached height at ({x}, {z}) with the camera at {cam_z}"
                    );
                }
            }
            cam_z += 1.0;
        }
    }
}
