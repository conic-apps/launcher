// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The hyperbola sky.
//!
//! Six hyperbolae (`x²/a² - y²/b² = 1`, a = 40 … 640, b = 200) are stroked in
//! all four branches, the whole set rotated -40° about a point a quarter of the
//! way down the window, each curve fainter than the last and all of them
//! anti-aliased at 0.6 CSS pixels wide. Below a band centred at 60.6% of the
//! height the sky is erased with a `destination-out` gradient, which is what
//! lets the hyperbolae run past the horizon and dissolve into the terrain
//! instead of being cut off at it.
//!
//! The result is cached: it only depends on the size and the palette, so it is
//! rebuilt when the window resizes or the flavour changes and uploaded once,
//! unlike the world's per-frame image.

use slint::{Rgba8Pixel, SharedPixelBuffer};

use super::raster;
use super::scene::LAYER_ALPHA;

/// The hyperbolae's semi-major axes, faintest last.
const CURVES: [f32; 6] = [40.0, 110.0, 180.0, 280.0, 420.0, 640.0];
/// The semi-minor axis shared by every curve.
const B: f32 = 200.0;
/// Stroke width in CSS pixels.
const LINE_WIDTH: f32 = 0.6;
/// The rotation of the whole figure, in degrees.
const ROTATION_DEGREES: f32 = -40.0;
/// Where the figure's centre sits vertically, as a fraction of the height.
const CENTER_Y_PERCENT: f32 = 0.25;
/// The stroke opacity of the first curve; each following one loses 0.055.
const LINE_ALPHA: f32 = 0.45;
const LINE_ALPHA_STEP: f32 = 0.055;
/// Height of the fade-out band, as a fraction of the window height.
const SKY_FADE_HEIGHT_RATIO: f32 = 0.14;
/// The band is centred at `height / 1.65` (60.6%), which is below the horizon.
const SKY_FADE_CENTER_DIVISOR: f32 = 1.65;

/// What the sky needs to be drawn: the buffer size in pixels and how many
/// buffer pixels one CSS pixel is.
pub struct SkyRequest {
    pub width: u32,
    pub height: u32,
    /// Buffer pixels per CSS pixel (`dpr × 1.08 ×` the render scale).
    pub scale: f32,
    /// Latte draws black lines, every other flavour white.
    pub dark: bool,
}

/// Renders the sky into a premultiplied RGBA buffer, with the layer's 0.3
/// opacity baked in like the world's.
pub fn render(request: &SkyRequest) -> SharedPixelBuffer<Rgba8Pixel> {
    let width = request.width.max(1);
    let height = request.height.max(1);
    let length = (width * height) as usize;
    let pixel_count = width as usize;

    // Coverage of the curve being stroked, and the alpha accumulated over the
    // curves already composited. Each curve is meant to be stroked in one path
    // and the paths composited with `source-over`; blending per segment instead
    // would double-count the joins, where the polyline's points crowd together
    // near the asymptote. Both are bytes — a 255th of accuracy is invisible on
    // a line this faint, and it is a quarter of the memory traffic.
    let mut coverage = vec![0u8; length];
    let mut accumulated = vec![0u8; length];

    let center = (width as f32 / 2.0, height as f32 * CENTER_Y_PERCENT);
    let radians = ROTATION_DEGREES.to_radians();
    let (sin, cos) = radians.sin_cos();
    // The sky is specified in CSS pixels, as the design is; here the buffer is
    // addressed directly, so everything is scaled by how many buffer pixels a
    // CSS pixel is.
    let scale = request.scale;
    let range = width.max(height) as f32 / scale * 1.5;

    // Curves are drawn one at a time, and only over the rows they actually
    // reach — they all live in the upper part of the window, so the passes that
    // walk the buffer skip most of it.
    let half_width = LINE_WIDTH * scale / 2.0;
    let mut top = height;
    let mut bottom = 0u32;

    for (index, a) in CURVES.iter().enumerate() {
        let mut curve_top = height;
        let mut curve_bottom = 0u32;
        for (sign_x, sign_y) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            let mut previous: Option<(f32, f32)> = None;
            let mut x = *a;
            while x <= range {
                let y = sign_y * B * ((x * x) / (a * a) - 1.0).sqrt();
                if y.is_finite() {
                    let real_x = sign_x * x;
                    // translate(center) then rotate, in CSS pixels, then the
                    // device scale.
                    let point = (
                        center.0 + (real_x * cos - y * sin) * scale,
                        center.1 + (real_x * sin + y * cos) * scale,
                    );
                    if let Some(from) = previous
                        && let Some((from_y, to_y)) =
                            draw_segment(&mut coverage, width, height, from, point, half_width)
                    {
                        curve_top = curve_top.min(from_y).min(to_y);
                        curve_bottom = curve_bottom.max(from_y).max(to_y);
                    }
                    previous = Some(point);
                }
                x += 1.0;
            }
        }
        if curve_bottom < curve_top {
            continue;
        }
        top = top.min(curve_top);
        bottom = bottom.max(curve_bottom);

        // `A = A + a·c·(1 - A)`, in fixed point: the source-over the
        // accumulated curves compose with.
        let line_alpha = ((LINE_ALPHA - index as f32 * LINE_ALPHA_STEP).max(0.0) * 255.0) as u32;
        for y in curve_top..=curve_bottom {
            let row = (y * width) as usize;
            for x in 0..width as usize {
                let cover = coverage[row + x] as u32;
                if cover == 0 {
                    continue;
                }
                let alpha = accumulated[row + x] as u32;
                let source = (line_alpha * cover * (255 - alpha)) >> 16;
                accumulated[row + x] = (alpha + source).min(255) as u8;
            }
        }
        for y in curve_top..=curve_bottom {
            let row = (y * width) as usize;
            coverage[row..row + width as usize].fill(0);
        }
    }

    // The `destination-out` gradient: transparent at the top of the band and
    // fully opaque at its bottom, then carried on to the bottom of the window.
    let band = height as f32 * SKY_FADE_HEIGHT_RATIO;
    let fade_top = height as f32 / SKY_FADE_CENTER_DIVISOR - band / 2.0;
    let fade_bottom = fade_top + band;
    // Line colour: white on the dark flavours, black on Latte.
    let line = if request.dark { 1.0 } else { 0.0 };

    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    if bottom < top {
        return buffer;
    }
    for (y, row) in buffer
        .make_mut_slice()
        .chunks_exact_mut(pixel_count)
        .enumerate()
        .take((bottom + 1) as usize)
        .skip(top as usize)
    {
        let fade = if (y as f32) <= fade_top {
            1.0
        } else if y as f32 >= fade_bottom {
            0.0
        } else {
            1.0 - (y as f32 - fade_top) / band
        };
        let faded = fade * LAYER_ALPHA;
        for (x, pixel) in row.iter_mut().enumerate() {
            let alpha = accumulated[y * pixel_count + x];
            if alpha == 0 {
                continue;
            }
            let packed = raster::color(line, line, line, f32::from(alpha) / 255.0 * faded);
            let bytes = packed.to_ne_bytes();
            *pixel = Rgba8Pixel {
                r: bytes[0],
                g: bytes[1],
                b: bytes[2],
                a: bytes[3],
            };
        }
    }
    buffer
}

/// Paints one anti-aliased segment into `coverage`, keeping the maximum — the
/// union of the segments is the stroked polyline — and returns the rows it
/// touched, so the passes that composite the curve can skip the rest.
#[must_use]
fn draw_segment(
    coverage: &mut [u8],
    width: u32,
    height: u32,
    from: (f32, f32),
    to: (f32, f32),
    half_width: f32,
) -> Option<(u32, u32)> {
    let reach = half_width + 1.0;
    if from.0.min(to.0) - reach >= width as f32
        || from.1.min(to.1) - reach >= height as f32
        || from.0.max(to.0) + reach < 0.0
        || from.1.max(to.1) + reach < 0.0
    {
        return None;
    }
    let min_x = (from.0.min(to.0) - reach)
        .floor()
        .clamp(0.0, width as f32 - 1.0) as u32;
    let min_y = (from.1.min(to.1) - reach)
        .floor()
        .clamp(0.0, height as f32 - 1.0) as u32;
    let max_x = (from.0.max(to.0) + reach)
        .ceil()
        .clamp(0.0, width as f32 - 1.0) as u32;
    let max_y = (from.1.max(to.1) + reach)
        .ceil()
        .clamp(0.0, height as f32 - 1.0) as u32;

    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length_squared = dx * dx + dy * dy;
    let inverse_length_squared = if length_squared > 0.0 {
        1.0 / length_squared
    } else {
        0.0
    };

    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            // Distance to the segment, clamped to its ends.
            let t = (((px - from.0) * dx + (py - from.1) * dy) * inverse_length_squared)
                .clamp(0.0, 1.0);
            let nearest_x = from.0 + dx * t;
            let nearest_y = from.1 + dy * t;
            let distance =
                ((px - nearest_x) * (px - nearest_x) + (py - nearest_y) * (py - nearest_y)).sqrt();
            let coverage_value = (half_width + 0.5 - distance).clamp(0.0, 1.0);
            if coverage_value <= 0.0 {
                continue;
            }
            let value = (coverage_value * 255.0 + 0.5) as u8;
            let index = (y * width + x) as usize;
            coverage[index] = coverage[index].max(value);
        }
    }
    Some((min_y, max_y))
}
