// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The world render pass: projects the `Scene`'s faces and draws them.
//!
//! Two halves: the fills (blended, writing depth, far to near in emission
//! order, with the ground quad first), then the outlines — screen-space quads
//! two device pixels wide, `LEQUAL`, replacing whatever they land on.

use slint::{Rgba8Pixel, SharedPixelBuffer};

use super::raster::{self, Canvas, Vertex};
use super::scene::{self, EDGE_WIDTH_DEVICE_PX, Scene};

/// Everything the world needs for one frame.
pub struct FrameRequest {
    /// Render target size in pixels (the window's device size times the render
    /// scale — see `background::controller::WORLD_PIXEL_BUDGET`).
    pub width: u32,
    pub height: u32,
    /// Buffer pixels per *device* pixel: the outline is two device pixels wide,
    /// so it shrinks with the render scale or the lines would thicken.
    pub edge_scale: f32,
    pub cam_z: f32,
    /// The palette's `crust` — both the fill colour and the base the outlines
    /// mix from.
    pub crust: [f32; 3],
    /// Latte outlines in black, every other flavour in white.
    pub dark: bool,
}

pub struct WorldRenderer {
    canvas: Canvas,
    scene: Scene,
    /// Projected faces, kept between the fill pass and the stroke pass so a
    /// face is projected once per frame.
    projected: Vec<[Vertex; 4]>,
}

impl WorldRenderer {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(1, 1),
            scene: Scene::new(),
            projected: Vec::new(),
        }
    }

    pub fn render(&mut self, request: &FrameRequest) -> SharedPixelBuffer<Rgba8Pixel> {
        let mut clock = Clock::start();
        self.canvas.resize(request.width, request.height);
        let half_w = self.canvas.width() as f32 / 2.0;
        let half_h = self.canvas.height() as f32 / 2.0;
        let focal = half_w / scene::FOV_HALF_TAN;
        let aspect = self.canvas.height() as f32 / self.canvas.width() as f32;

        // The geometry only changes when the camera crosses a block boundary;
        // between rebuilds this is a no-op and only the projection runs.
        self.scene.update(request.cam_z, aspect, false);

        self.canvas.begin();
        clock.mark("begin");

        let project = |corner: [f32; 3]| project(corner, request.cam_z, focal, half_w, half_h);

        // The ground slab goes down first, at the far plane, and the terrain
        // blends over it (which the depth test allows because it is `LEQUAL`).
        let ground: [Vertex; 4] = std::array::from_fn(|i| project(self.scene.ground[i]));
        self.canvas.fill_quad(
            &ground,
            raster::color(
                request.crust[0],
                request.crust[1],
                request.crust[2],
                scene::LAYER_ALPHA,
            ),
        );

        self.projected.clear();
        self.projected.extend(
            self.scene
                .faces
                .iter()
                .map(|face| std::array::from_fn(|i| project(face.corners[i]))),
        );
        clock.mark("ground+project");

        for (face, quad) in self.scene.faces.iter().zip(self.projected.iter()) {
            let packed = raster::color(
                request.crust[0],
                request.crust[1],
                request.crust[2],
                face.fill_alpha * scene::LAYER_ALPHA,
            );
            self.canvas.fill_quad(quad, packed);
        }
        clock.mark("fills");

        // Line colour: the flavours that draw on a dark crust outline in white,
        // and Latte in black (`const c = latte ? 0 : 255`). The fills are the
        // crust itself, so in a dark palette these outlines are the whole
        // picture.
        let outline = if request.dark { 1.0 } else { 0.0 };
        let half_edge = EDGE_WIDTH_DEVICE_PX * request.edge_scale / 2.0;
        for (face, quad) in self.scene.faces.iter().zip(self.projected.iter()) {
            let mix = face.edge_mix;
            let packed = raster::color(
                request.crust[0] + (outline - request.crust[0]) * mix,
                request.crust[1] + (outline - request.crust[1]) * mix,
                request.crust[2] + (outline - request.crust[2]) * mix,
                scene::LAYER_ALPHA,
            );
            for i in 0..4 {
                let a = quad[i];
                let b = quad[(i + 1) % 4];
                // The GPU path expands the edge into a quad in the vertex
                // shader, in a y-up pixel space, so the normal is computed
                // there and only the result comes back here.
                let dx = b.x - a.x;
                let dy = -(b.y - a.y);
                let (nx, ny) = (-dy, dx);
                let length = (nx * nx + ny * ny).sqrt();
                if length <= 1e-5 {
                    continue;
                }
                let (ox, oy) = (nx / length * half_edge, ny / length * half_edge);
                // Perimeter order, not the vertex-shader order: the four
                // corners are the band's two long sides, and visiting them
                // a+ a- b+ b- would give the rasteriser a bowtie.
                let corners = [
                    Vertex {
                        x: a.x + ox,
                        y: a.y - oy,
                        inv_depth: a.inv_depth,
                    },
                    Vertex {
                        x: b.x + ox,
                        y: b.y - oy,
                        inv_depth: b.inv_depth,
                    },
                    Vertex {
                        x: b.x - ox,
                        y: b.y + oy,
                        inv_depth: b.inv_depth,
                    },
                    Vertex {
                        x: a.x - ox,
                        y: a.y + oy,
                        inv_depth: a.inv_depth,
                    },
                ];
                self.canvas.edge_quad(&corners, packed);
            }
        }

        clock.mark("edges");
        clock.report(self.scene.faces.len());
        self.canvas.finish()
    }
}

/// Per-pass timings, logged at debug level with `RUST_LOG=background=debug`.
/// The renderer is the one thing in the app whose cost is worth watching while
/// it runs, and the split says which pass to look at when it grows.
struct Clock {
    started: std::time::Instant,
    last: std::time::Instant,
    marks: [(&'static str, f64); 5],
    count: usize,
}

impl Clock {
    fn start() -> Self {
        let now = std::time::Instant::now();
        Self {
            started: now,
            last: now,
            marks: [("", 0.0); 5],
            count: 0,
        }
    }

    fn mark(&mut self, label: &'static str) {
        let now = std::time::Instant::now();
        if self.count < self.marks.len() {
            self.marks[self.count] = (label, (now - self.last).as_secs_f64() * 1000.0);
            self.count += 1;
        }
        self.last = now;
    }

    fn report(&self, faces: usize) {
        if !log::log_enabled!(target: "background", log::Level::Debug) {
            return;
        }
        let marks: Vec<String> = self.marks[..self.count]
            .iter()
            .map(|(label, ms)| format!("{label} {ms:.1}ms"))
            .collect();
        log::debug!(
            target: "background",
            "world frame: {:.1}ms total ({}) — {} faces",
            self.started.elapsed().as_secs_f64() * 1000.0,
            marks.join(", "),
            faces,
        );
    }
}

impl Default for WorldRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// The vertex shader's projection: `ndcX = (x - camX) * focal / dz / halfW` and
/// the same for y with the sign flipped, mapped into window pixels.
///
/// The depth is clamped to the near and far planes before being inverted, which
/// is the shader's `clamp(dz, uNearZ, uFarZ)` as it reaches the depth test.
#[inline]
fn project(corner: [f32; 3], cam_z: f32, focal: f32, half_w: f32, half_h: f32) -> Vertex {
    let dz = corner[2] - cam_z;
    let scale = focal / dz;
    Vertex {
        x: half_w + (corner[0] - scene::CAM_X) * scale,
        y: half_h - (corner[1] - scene::CAM_Y) * scale,
        inv_depth: 1.0 / dz.clamp(scene::NEAR_PLANE, scene::FADE_END),
    }
}
