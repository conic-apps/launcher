// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A z-buffered software rasteriser for the window background.
//!
//! The Vue renders the 3D world with WebGL2 (`WindowBackground.vue`); Slint
//! 1.18 has no custom-shader hook, so the same pipeline runs here on the CPU
//! instead. Only two primitives are needed, and they are the original's two
//! draw calls:
//!
//! * [`Canvas::fill_quad`] — the world pass: convex quads (one per block face,
//!   split into the two triangles the original's index buffer names: `(0,1,2)`
//!   and `(0,2,3)`) blended premultiplied source-over into the buffer, writing
//!   depth. The faces are emitted far-to-near, exactly as there, so a nearer
//!   face blends over a farther one covering the same pixel.
//! * [`Canvas::edge_quad`] — the stroke pass: the same quads, expanded to the
//!   2px screen-space outlines, drawn afterwards with a *less-or-equal* depth
//!   test, blending disabled (they overwrite) and no depth write — the
//!   original's `disable(BLEND); depthFunc(LEQUAL); depthMask(false);`.
//!
//! Depth is stored as `1 / dz`, where `dz` is the vertex's distance along the
//! view axis. The projection divides by `dz`, so `1 / dz` is what varies
//! *linearly* across a planar quad in screen space: interpolating it
//! barycentrically reproduces what the GPU's perspective-correct interpolation
//! hands the depth test. Comparing reciprocals inverts the test — "greater or
//! equal" is the original's `LEQUAL` — and a cleared buffer of zeroes means
//! "infinitely far", which is where a cleared depth buffer starts.
//!
//! Triangles are rasterised as per-row spans: each row's x interval comes from
//! solving the three edge functions for x, and inside it the edge values and
//! the depth advance by a constant per pixel. Walking a bounding box instead
//! costs a thin, steep quad its whole rectangle — which is most of the outlines
//! and the fill of every face seen at a grazing angle.
//!
//! The original's fragment shaders also `discard` outside the window's bottom
//! two rounded corners (a CSS `border-radius` cannot clip a GPU-composited
//! WebGL layer). That is not ported: the window's container in `app.slint`
//! already clips the background to its 16px radius, and the canvas there is
//! larger than the window (the parallax wrapper scales it by 1.08), so the
//! original's own discard lands 4% outside the visible corners anyway.

use slint::{Rgba8Pixel, SharedPixelBuffer};

/// A projected vertex: window coordinates in device pixels (y downwards, the
/// horizon at `height / 2`) plus the reciprocal view depth used for the test.
#[derive(Clone, Copy, Default, Debug)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub inv_depth: f32,
}

/// Packs a premultiplied colour the way [`Rgba8Pixel`] stores it.
///
/// `from_ne_bytes` keeps this endian-neutral: the bytes are laid out r, g, b, a
/// in memory exactly like the pixel struct, so writing a packed colour into the
/// buffer is a single 32-bit store.
#[inline]
pub fn color(r: f32, g: f32, b: f32, alpha: f32) -> u32 {
    let channel = |value: f32| (value * alpha * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
    u32::from_ne_bytes([
        channel(r),
        channel(g),
        channel(b),
        (alpha * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
    ])
}

/// How a rasterised quad meets what is already in the buffer.
#[derive(Clone, Copy, PartialEq)]
enum Write {
    /// Premultiplied source-over, writing depth — the fill pass.
    Blend,
    /// Replace, depth-tested but not written — the stroke pass.
    Overwrite,
}

/// The offscreen buffer the world and the sky are drawn into.
///
/// The pixel buffer is double-buffered: the finished frame is handed to Slint by
/// value (a cheap refcount bump) and the next frame is drawn into the other one,
/// so a buffer Slint still holds is never written to — which would make
/// `make_mut_slice` copy the whole thing.
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: [SharedPixelBuffer<Rgba8Pixel>; 2],
    depth: Vec<f32>,
    parity: usize,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            pixels: [
                SharedPixelBuffer::new(width, height),
                SharedPixelBuffer::new(width, height),
            ],
            depth: vec![0.0; (width * height) as usize],
            // Starts at 1 so the first `begin` swaps in buffer 0.
            parity: 1,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Rebuilds the buffers for a new size.
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(width, height);
    }

    /// Starts a frame: clears colour to transparent and depth to "infinitely
    /// far", and swaps in the write buffer.
    pub fn begin(&mut self) {
        self.parity ^= 1;
        for pixel in self.pixels[self.parity].make_mut_slice() {
            *pixel = Rgba8Pixel::default();
        }
        // 1/dz = 0 is dz = infinity — a cleared depth buffer — and zeroing is a
        // plain `memset`.
        self.depth.fill(0.0);
    }

    /// The frame just drawn, as a premultiplied pixel buffer. Slint turns this
    /// into an `Image` on the UI thread.
    pub fn finish(&self) -> SharedPixelBuffer<Rgba8Pixel> {
        self.pixels[self.parity].clone()
    }

    /// Blends one convex quad into the colour buffer and writes depth.
    pub fn fill_quad(&mut self, quad: &[Vertex; 4], packed: u32) {
        // The original's index buffer draws `(0,1,2)` and `(0,2,3)`; the two
        // triangles meet on the `(0,2)` diagonal, where the pixel centres fall
        // on one side or the other and so are painted once.
        self.quad(quad, packed, Write::Blend);
    }

    /// Overwrites one convex quad's pixels where the depth test passes, without
    /// writing depth — the stroke pass.
    ///
    /// An outline is drawn as its own quad, so its interpolated depth and the
    /// depth the face left behind are two roundings of the same plane. The
    /// tolerance is what keeps a shared boundary from coming out dotted; it is
    /// far smaller than the gap between any two faces (blocks are a whole unit
    /// apart).
    pub fn edge_quad(&mut self, quad: &[Vertex; 4], packed: u32) {
        self.quad(quad, packed, Write::Overwrite);
    }

    fn quad(&mut self, quad: &[Vertex; 4], packed: u32, mode: Write) {
        // The second triangle's first edge is the diagonal the two share. It is
        // tested strictly, so a pixel whose centre lands exactly on it is
        // painted by one of them and not twice — a doubled blend would draw a
        // faint seam down every translucent face.
        self.triangle([quad[0], quad[1], quad[2]], packed, mode, false);
        self.triangle([quad[0], quad[2], quad[3]], packed, mode, true);
    }

    /// Rasterises one triangle as per-row spans.
    ///
    /// The triangle is scanned along whichever axis it is longer, so a thin,
    /// steep quad — a wall seen edge-on, or a block outline — costs roughly its
    /// length instead of the area of its bounding box. The row coordinate is
    /// `u` and the span runs along `v`; in buffer memory the pixel index is
    /// `u * width + v` when `v` is x, and `v * width + u` when it is y.
    fn triangle(
        &mut self,
        triangle: [Vertex; 3],
        packed: u32,
        mode: Write,
        exclusive_diagonal: bool,
    ) {
        const DEPTH_EPSILON: f32 = 1e-5;
        let width = self.width;
        let height = self.height;

        let min_x = triangle[0].x.min(triangle[1].x).min(triangle[2].x);
        let max_x = triangle[0].x.max(triangle[1].x).max(triangle[2].x);
        let min_y = triangle[0].y.min(triangle[1].y).min(triangle[2].y);
        let max_y = triangle[0].y.max(triangle[1].y).max(triangle[2].y);
        if !min_x.is_finite()
            || !min_y.is_finite()
            || max_x < 0.0
            || max_y < 0.0
            || min_x >= width as f32
            || min_y >= height as f32
        {
            return;
        }

        let transpose = (max_y - min_y) > (max_x - min_x);
        let (u_limit, v_limit) = if transpose {
            (width, height)
        } else {
            (height, width)
        };
        let mut scan = [[0.0f32; 3]; 3];
        for (index, vertex) in triangle.iter().enumerate() {
            scan[index] = if transpose {
                [vertex.x, vertex.y, vertex.inv_depth]
            } else {
                [vertex.y, vertex.x, vertex.inv_depth]
            };
        }

        // `e(u, v) = a·u + b·v + c` for each edge, positive inside whichever
        // way round the vertices come.
        let mut edges = [[0.0f32; 3]; 3];
        for index in 0..3 {
            let from = scan[index];
            let to = scan[(index + 1) % 3];
            let (du, dv) = (to[0] - from[0], to[1] - from[1]);
            edges[index] = [dv, -du, du * from[1] - dv * from[0]];
        }
        let area = edges[0][0] * scan[2][0] + edges[0][1] * scan[2][1] + edges[0][2];
        if area == 0.0 {
            return;
        }
        let inside_sign = if area > 0.0 { 1.0 } else { -1.0 };
        let inv_area = 1.0 / area;
        // Depth per unit of each edge's value: a vertex's barycentric weight is
        // the opposite edge's value over the area, so the depth is
        // `Σ e_opposite · inv_depth · inv_area` — no division per pixel.
        // The edge values below carry `inside_sign`, so the weights have to as
        // well for the depth to keep its direction on either winding.
        let weight_0 = scan[0][2] * inv_area * inside_sign;
        let weight_1 = scan[1][2] * inv_area * inside_sign;
        let weight_2 = scan[2][2] * inv_area * inside_sign;

        let min_u = scan[0][0].min(scan[1][0]).min(scan[2][0]);
        let max_u = scan[0][0].max(scan[1][0]).max(scan[2][0]);
        // The rows whose pixel centres fall inside the triangle's own extent.
        // The row past its last vertex has no edge bounding it — the span solve
        // there stretches to the whole width, and while the per-pixel test
        // would reject those pixels, the walk itself is what costs.
        let first_u = (min_u - 0.5).ceil().clamp(0.0, u_limit as f32 - 1.0) as u32;
        let last_u = (max_u - 0.5).floor().clamp(0.0, u_limit as f32 - 1.0) as u32;

        // Each edge's v where it crosses a row's pixel centres, as
        // `v = step · u + start` — solved once, advanced by `step` per row. An
        // edge with no `v` term is parallel to the rows and constrains the
        // whole of each one.
        let mut steps = [0.0f32; 3];
        let mut starts = [0.0f32; 3];
        // The rows an edge actually spans. A triangle's row is crossed by two
        // of its edges; the third's line carries on past its own end, and
        // letting it bound a row it never reaches would cut the span down (or
        // extend it) to something the triangle does not cover.
        let mut rows = [(0.0f32, 0.0f32); 3];
        let (e0a, e0b, e0c) = (edges[0][0], edges[0][1], edges[0][2]);
        let (e1a, e1b, e1c) = (edges[1][0], edges[1][1], edges[1][2]);
        let (e2a, e2b, e2c) = (edges[2][0], edges[2][1], edges[2][2]);
        for index in 0..3 {
            let [a, b, c] = edges[index];
            if b != 0.0 {
                steps[index] = -a / b;
                starts[index] = -c / b;
            }
            let from = scan[index][0];
            let to = scan[(index + 1) % 3][0];
            rows[index] = (from.min(to), from.max(to));
        }

        let alpha = (packed >> 24) & 0xff;
        let source_rgb = packed & 0x00ff_ffff;
        let depth = &mut self.depth;
        let pixels = self.pixels[self.parity].make_mut_slice();

        for u in first_u..=last_u {
            let center_u = u as f32 + 0.5;
            let mut low = f32::NEG_INFINITY;
            let mut high = f32::INFINITY;
            let mut skip = false;
            for (index, edge) in edges.iter().enumerate() {
                let [a, b, c] = *edge;
                if b == 0.0 {
                    // Parallel to the rows: it either holds for the whole row
                    // or none of it.
                    if (a * center_u + c) * inside_sign < 0.0 {
                        skip = true;
                        break;
                    }
                    continue;
                }
                let (row_from, row_to) = rows[index];
                if center_u < row_from || center_u > row_to {
                    continue;
                }
                let bound = steps[index] * center_u + starts[index];
                if b * inside_sign > 0.0 {
                    low = low.max(bound);
                } else {
                    high = high.min(bound);
                }
            }
            if skip {
                continue;
            }
            // Pixel centres are at `v + 0.5`.
            let first_v = (low - 0.5).ceil().clamp(0.0, v_limit as f32 - 1.0) as u32;
            let last_v = (high - 0.5).floor().min(v_limit as f32 - 1.0);
            if last_v < 0.0 || first_v as f32 > last_v {
                continue;
            }
            let last_v = last_v as u32;

            let mut e0 = (e0a * center_u + e0b * (first_v as f32 + 0.5) + e0c) * inside_sign;
            let mut e1 = (e1a * center_u + e1b * (first_v as f32 + 0.5) + e1c) * inside_sign;
            let mut e2 = (e2a * center_u + e2b * (first_v as f32 + 0.5) + e2c) * inside_sign;
            // The span runs along `v`, so that is the coefficient the edge
            // values advance by.
            let (se0, se1, se2) = (e0b * inside_sign, e1b * inside_sign, e2b * inside_sign);
            let mut index = if transpose {
                first_v as usize * width as usize + u as usize
            } else {
                u as usize * width as usize + first_v as usize
            };
            let index_step = if transpose { width as usize } else { 1 };

            for _ in first_v..=last_v {
                let inside_first = if exclusive_diagonal {
                    e0 > 0.0
                } else {
                    e0 >= 0.0
                };
                if inside_first && e1 >= 0.0 && e2 >= 0.0 {
                    let inv = e1 * weight_0 + e2 * weight_1 + e0 * weight_2;
                    match mode {
                        Write::Blend => {
                            if inv >= depth[index] {
                                depth[index] = inv;
                                let destination = read(&pixels[index]);
                                // Nothing under this pixel yet — the common
                                // case, since the terrain's faces tile rather
                                // than overlap.
                                pixels[index] = unpack(if destination >> 24 == 0 {
                                    packed
                                } else {
                                    blend_over(destination, source_rgb, alpha)
                                });
                            }
                        }
                        Write::Overwrite => {
                            if inv + DEPTH_EPSILON >= depth[index] {
                                pixels[index] = unpack(packed);
                            }
                        }
                    }
                }
                e0 += se0;
                e1 += se1;
                e2 += se2;
                index += index_step;
            }
        }
    }
}

/// `dst = src + dst * (1 - alpha)`, per channel, in the premultiplied buffers.
#[inline]
fn blend_over(destination: u32, source_rgb: u32, alpha: u32) -> u32 {
    let channel = |shift: u32| -> u32 {
        let dst = (destination >> shift) & 0xff;
        let src = (source_rgb >> shift) & 0xff;
        // `dst - (dst * alpha >> 8)` trades one 255th of accuracy for a shift
        // in place of a division; at these alpha values the error stays below
        // one 8-bit step.
        let blended = dst - ((dst * alpha) >> 8);
        (src + blended).min(255) << shift
    };
    let destination_alpha = (destination >> 24) & 0xff;
    let alpha = alpha + (destination_alpha - ((destination_alpha * alpha) >> 8));
    channel(0) | channel(8) | channel(16) | (alpha.min(255) << 24)
}

#[inline]
fn unpack(value: u32) -> Rgba8Pixel {
    let bytes = value.to_ne_bytes();
    Rgba8Pixel {
        r: bytes[0],
        g: bytes[1],
        b: bytes[2],
        a: bytes[3],
    }
}

#[inline]
fn read(pixel: &Rgba8Pixel) -> u32 {
    u32::from_ne_bytes([pixel.r, pixel.g, pixel.b, pixel.a])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32, y: f32, inv_depth: f32) -> Vertex {
        Vertex { x, y, inv_depth }
    }

    /// A quad covering a known area fills it, paint included — and the two
    /// triangles do not both paint the diagonal between them.
    #[test]
    fn fills_a_quad_once() {
        let mut canvas = Canvas::new(64, 64);
        canvas.begin();
        let quad = [
            vertex(8.0, 8.0, 1.0),
            vertex(56.0, 16.0, 1.0),
            vertex(40.0, 56.0, 1.0),
            vertex(12.0, 40.0, 1.0),
        ];
        // Alpha 1.0 (opaque in the buffer, whatever the layer does with it), so
        // a pixel painted twice stands out immediately.
        canvas.fill_quad(&quad, color(1.0, 1.0, 1.0, 1.0));
        let pixels = canvas.finish();
        let painted = pixels.as_slice().iter().filter(|p| p.a > 0).count();
        // The quad's area by the shoelace formula; a pixel centre inside it is
        // painted, so the count should be close to it.
        let area: f32 = (0..4)
            .map(|i| {
                let a = quad[i];
                let b = quad[(i + 1) % 4];
                a.x * b.y - b.x * a.y
            })
            .sum::<f32>()
            .abs()
            / 2.0;
        assert!(
            (painted as f32 - area).abs() < area * 0.1,
            "painted {painted} pixels for an area of {area}"
        );

        let mut canvas = Canvas::new(64, 64);
        canvas.begin();
        canvas.fill_quad(&quad, color(1.0, 1.0, 1.0, 0.5));
        let half = canvas.finish();
        let doubled = half
            .as_slice()
            .iter()
            .filter(|p| p.a != 128 && p.a != 0)
            .count();
        assert_eq!(doubled, 0, "a pixel was painted twice along the diagonal");
    }

    /// A thin band — what every block outline is — paints its length and not
    /// its bounding box.
    #[test]
    fn paints_a_thin_band() {
        for (dx, dy) in [
            (100.0f32, 10.0f32),
            (10.0, 100.0),
            (100.0, 100.0),
            (0.0, 100.0),
            (100.0, 0.0),
        ] {
            let length = (dx * dx + dy * dy).sqrt();
            // A 2px band from (20, 20) to (20+dx, 20+dy).
            let (nx, ny) = (-dy / length, dx / length);
            let (ox, oy) = (nx * 1.0, ny * 1.0);
            let quad = [
                vertex(20.0 + ox, 20.0 + oy, 1.0),
                vertex(20.0 + dx + ox, 20.0 + dy + oy, 1.0),
                vertex(20.0 + dx - ox, 20.0 + dy - oy, 1.0),
                vertex(20.0 - ox, 20.0 - oy, 1.0),
            ];
            let mut canvas = Canvas::new(200, 200);
            canvas.begin();
            canvas.edge_quad(&quad, color(1.0, 1.0, 1.0, 1.0));
            let pixels = canvas.finish();
            let painted = pixels.as_slice().iter().filter(|p| p.a > 0).count();
            let expected = length * 2.0;
            assert!(
                (painted as f32 - expected).abs() < expected * 0.5,
                "band ({dx}, {dy}): painted {painted}, expected about {expected}"
            );
        }
    }

    /// Depth decides: a farther quad drawn afterwards leaves the nearer one
    /// alone, and the outline pass replaces what it lands on.
    #[test]
    fn depth_orders_the_passes() {
        let quad = |inv| {
            [
                vertex(8.0, 8.0, inv),
                vertex(56.0, 8.0, inv),
                vertex(56.0, 56.0, inv),
                vertex(8.0, 8.0, inv),
            ]
        };
        let mut canvas = Canvas::new(64, 64);
        canvas.begin();
        canvas.fill_quad(&quad(2.0), color(1.0, 0.0, 0.0, 1.0));
        canvas.fill_quad(&quad(1.0), color(0.0, 1.0, 0.0, 1.0));
        let pixels = canvas.finish();
        let centre = pixels.as_slice()[(32 * 64 + 32) as usize];
        assert_eq!((centre.r, centre.g), (255, 0), "the farther quad won");
        assert!(centre.a > 0);

        // The stroke pass overwrites the fill where its depth allows.
        let mut canvas = Canvas::new(64, 64);
        canvas.begin();
        canvas.fill_quad(&quad(1.0), color(1.0, 0.0, 0.0, 1.0));
        canvas.edge_quad(&quad(1.0), color(0.0, 0.0, 1.0, 1.0));
        let pixels = canvas.finish();
        let centre = pixels.as_slice()[(32 * 64 + 32) as usize];
        assert_eq!(
            (centre.r, centre.b),
            (0, 255),
            "the outline did not replace"
        );
    }
}
