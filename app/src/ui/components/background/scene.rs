// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The 3D background world — terrain, trees and face culling.
//!
//! Nothing about the *scene* is WebGL-specific, so this module keeps the same
//! functions and constants (and their names) and emits the face list along with
//! the GPU vertex and index buffers. The face list is rebuilt when the camera
//! crosses into a new block, the aspect changes or the buffer set changes — the
//! ring height cache and the per-rebuild row invalidation are load-bearing: the
//! ring maps z's 128 apart to the same slot.
//!
//! Everything a face needs to be drawn (four world-space corners, its fill
//! alpha and its edge mix) is baked here, because those depend on the camera's
//! *block*, not on its exact position; the projection is redone every frame.

use std::collections::HashSet;

/// Camera advance speed in blocks per second.
pub const CAMERA_SPEED: f32 = 2.0;
/// The tangent of half the 60° field of view — the frustum's slope in x per
/// unit z.
pub const FOV_HALF_TAN: f32 = 0.577_350_26; // tan(60° / 2)
/// The near plane the depth buffer clamps to (`uNearZ`).
pub const NEAR_PLANE: f32 = 0.2;
/// World render distance in blocks.
pub const VIEW_DISTANCE: i32 = 120;
/// The distance at which the fog starts eating into the fills (`FADE_START`).
pub const FADE_START: f32 = VIEW_DISTANCE as f32 * 0.55;
/// The far plane; everything beyond it is dropped (`FADE_END`).
pub const FADE_END: f32 = VIEW_DISTANCE as f32;

/// The camera's fixed position: x is between two block columns so the world
/// scrolls past evenly, y is one eye height above `BASE_HEIGHT`.
pub const CAM_X: f32 = -0.5;
pub const CAM_Y: f32 = BASE_HEIGHT as f32 + EYE_HEIGHT;

/// The overlay opacity of the world layer (`.world { opacity: 0.3 }`).
pub const LAYER_ALPHA: f32 = 0.3;

/// The outline width, in *device* pixels. The outlines are expanded into quads
/// in screen space, so this is a width on screen rather than in the world.
pub const EDGE_WIDTH_DEVICE_PX: f32 = 2.0;

/// Surface height the hills are generated around.
const BASE_HEIGHT: i32 = 4;
/// Hill amplitude in blocks.
const HILL_AMPLITUDE: f32 = 2.2;
/// Camera height above `BASE_HEIGHT`.
const EYE_HEIGHT: f32 = 10.6;
/// Chance that a column grows a tree.
const TREE_DENSITY: f64 = 0.0007;
/// Block outline opacity at zero distance.
const WORLD_STROKE_ALPHA: f32 = 0.55;
/// Fill opacity at zero and full distance.
const FILL_ALPHA_NEAR: f32 = 1.0;
const FILL_ALPHA_FAR: f32 = 0.15;

/// The block column the camera travels down: `floor(CAM_X)`, `floor(CAM_Y)`.
/// A tree whose blocks would occupy it is not grown, which is what
/// `tree_hits_camera` is for. (With `CAM_Y = 14.6` and trees topping out at
/// `y = 13`, no tree can reach it — the check is kept, and would matter if the
/// camera height ever changed.)
const CAM_X_CELL: i32 = -1;
const CAM_Y_CELL: i32 = 14;

/// Half the widest row of columns the frustum can see, plus a margin
/// (`MAX_HALF_X`): columns `CAM_X ± 74`, i.e. x from -75 to 74.
const MAX_HALF_X: i32 = 74;

/// Trees reach two blocks sideways from their column; used to keep the
/// frustum cull conservative.
const TREE_OVERHANG: f32 = 3.0;

/// First x held by `heights`.
const HEIGHT_X_OFF: i32 = -79; // floor(CAM_X - MAX_HALF_X) - 4
const HEIGHT_X_SPAN: i32 = 164; // 2 * MAX_HALF_X + 16
const HEIGHT_Z_SPAN: i32 = 128; // VIEW_DISTANCE + 8

/// The eight corners of a unit block; the face constants below index into it.
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

/// Corner order of each visible face (all legal quads). The bottom face still
/// exists, though nothing can see it: the camera is above every block.
const FACE_NZ: [usize; 4] = [0, 1, 3, 2];
const FACE_NX: [usize; 4] = [0, 2, 6, 4];
const FACE_PX: [usize; 4] = [1, 5, 7, 3];
const FACE_PY: [usize; 4] = [2, 3, 7, 6];
const FACE_NY: [usize; 4] = [0, 1, 5, 4];

/// Tree shape: `[dx, dy, dz]` from the block above the surface. Editing this
/// list reshapes every tree.
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

/// One quad to draw, in world space.
#[derive(Clone, Copy)]
pub struct Face {
    pub corners: [[f32; 3]; 4],
    /// Fill opacity at this face's distance.
    pub fill_alpha: f32,
    /// How far the outline is mixed toward the edge colour (0 at the fog end).
    pub edge_mix: f32,
}

/// The world's geometry for one camera block.
pub struct Scene {
    /// Ring-buffered column heights; 0 means "not computed" (heights are 2..7).
    heights: Vec<f32>,
    /// Every block a tree occupies in the visible area, for occlusion tests.
    tree_blocks: HashSet<(i32, i32, i32)>,
    /// Faces in emission order: z rows far to near, x ascending, and within a
    /// block front, left, right, top — the order they are uploaded in, which
    /// decides how overlapping fills and tied edges resolve.
    pub faces: Vec<Face>,
    /// The far-plane quad covering everything below the horizon, emitted first.
    pub ground: [[f32; 3]; 4],
    /// The GPU path's buffers: fill vertices are `x y z alpha` with six indices
    /// per quad, and each outline is six
    /// `x y z mix other_x other_y other_z side` vertices — the screen-space
    /// expansion happens in the vertex shader. Built only when a GPU renderer is
    /// going to draw them.
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

    /// Rebuilds when the camera block, the aspect or the buffer set changed;
    /// returns whether it did.
    pub fn update(&mut self, cam_z: f32, aspect: f32, buffers: bool) -> bool {
        let floor = cam_z.floor() as i32;
        // The aspect decides how far down the ground quad reaches, so a window
        // resize has to rebuild even when the camera has not moved.
        let aspect_changed = (aspect - self.built_aspect).abs() > 1e-4;
        if self.built_floor == Some(floor) && self.buffers == buffers && !aspect_changed {
            return false;
        }
        self.built_floor = Some(floor);
        self.built_aspect = aspect;
        self.buffers = buffers;
        self.build(cam_z, aspect);
        true
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
        let x_reach = FADE_END * FOV_HALF_TAN;
        let y_reach = FADE_END * aspect * FOV_HALF_TAN;
        let wz = cam_z + FADE_END;
        self.ground = [
            [CAM_X - x_reach, CAM_Y, wz],
            [CAM_X + x_reach, CAM_Y, wz],
            [CAM_X - x_reach, CAM_Y - y_reach, wz],
            [CAM_X + x_reach, CAM_Y - y_reach, wz],
        ];
        if self.buffers {
            // Alpha 1: it is the slab that hides the sky below the horizon.
            for corner in self.ground {
                self.fill_vertices
                    .extend_from_slice(&[corner[0], corner[1], corner[2], 1.0]);
            }
            self.fill_indices.extend_from_slice(&[0, 1, 2, 1, 3, 2]);
        }

        // Collect the visible area's tree blocks first: they take part in the
        // occlusion tests the columns (and the trees themselves) run.
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

    /// Whether a column can project into the viewport at all while this
    /// geometry is in use.
    ///
    /// Off-screen columns would cost more than the visible ones, so they are
    /// skipped here — the image is identical, because a column outside the
    /// frustum projects outside it for every camera position in this block. The
    /// frustum widens with distance, so the test uses the block's *nearest*
    /// camera position and the furthest edge of the column (plus the tree
    /// overhang).
    fn column_may_be_visible(&self, x: i32, z: i32, cam_z: f32) -> bool {
        let dz = (z - cam_z.floor() as i32 - 1) as f32;
        let reach = FOV_HALF_TAN * dz.max(0.0) + TREE_OVERHANG;
        (x as f32 + 1.0 + TREE_OVERHANG) >= CAM_X - reach
            && (x as f32 - TREE_OVERHANG) <= CAM_X + reach
    }

    /// A deterministic hash of a lattice point.
    ///
    /// Implemented operation for operation (`Math.imul` is `wrapping_mul`, `>>>`
    /// is a shift on `u32`) and evaluated in `f64` like JavaScript's numbers, so
    /// the terrain reproduces the golden table in the tests exactly.
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

    /// Two octaves of value noise, rounded to whole blocks and clamped to the
    /// height range the camera and the fog are tuned for.
    fn terrain_height(x: i32, z: i32) -> i32 {
        let x = f64::from(x);
        let z = f64::from(z);
        let n1 = (Self::value_noise(x / 14.0, z / 14.0, 0x71) - 0.5) * 2.0;
        let n2 = (Self::value_noise(x / 4.5, z / 4.5, 0x3a) - 0.5) * 1.2;
        let h = (f64::from(BASE_HEIGHT) + n1 * f64::from(HILL_AMPLITUDE) + n2).round() as i32;
        h.clamp(2, 7)
    }

    /// `heightAt`: the ring cache keeps the terrain from being re-generated
    /// every time a column is asked about — which is several times per column,
    /// for the column and its three neighbours.
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

    /// Clears the cache slots of the row the camera is about to reach.
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

    /// Whether any of a tree's blocks would land in the column the camera
    /// travels down.
    fn tree_hits_camera(x: i32, h: i32) -> bool {
        let y0 = h + 1;
        TREE_SHAPE
            .iter()
            .any(|[dx, dy, _]| x + dx == CAM_X_CELL && y0 + dy == CAM_Y_CELL)
    }

    /// Whether a block is solid: at or below the surface, or part of a tree.
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

    /// Only faces that look at the camera and are not covered by a neighbouring
    /// block are emitted.
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

    /// `(x, z, height, hash2i(x, z, 0x5eed))` reference values, generated from a
    /// JavaScript implementation run through Node. The terrain must agree with
    /// them value for value, so this is asserted rather than eyeballed.
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

    /// The ring cache hands back a stored height instead of re-generating it,
    /// so it has to agree with the generator for every column the camera
    /// actually visits — including across the wrap where the 128th row reuses
    /// the first's slots. Advancing one block at a time is how `build` drives
    /// it, and the invalidation of the row the camera is about to reach is what
    /// keeps a stale value from being handed back.
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
