// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The skin-model and cape images the account view draws.
//!
//! Both are composed here rather than in Slint, which cannot crop or sample an
//! image itself. The model is the *front view* of the 64x64 player skin: the
//! head, torso, arms and legs, each drawn base layer first and its overlay
//! (hat / jacket / sleeves / trousers) blended on top, all nearest-neighbour.
//! This is the same artwork the Vue build drew into a `<canvas>`; producing a
//! pixel buffer is the Slint equivalent.
//!
//! The cape is the 10x17 face at the top-left of a 64x32 cape texture, scaled
//! to fit.

use image::RgbaImage;
use slint::{Image, SharedPixelBuffer};

/// Pixels per skin pixel in the rendered model; the model is 16x32 skin pixels.
const UNIT: u32 = 4;

/// Whether `skin` is a slim (3px-arm) model.
///
/// A skin does not carry that flag in its pixels except by the width of the arm
/// region: the fourth arm column of a classic skin is opaque, and a slim skin
/// leaves it transparent. `variant`/`model` metadata is used first where the
/// account carries it; this is the fallback for a texture with none.
pub fn detect_slim(skin: &RgbaImage) -> bool {
    let scale = skin.width() as f32 / 64.0;
    let sample = |x: f32, y: f32| -> u8 {
        let x = (x * scale) as u32;
        let y = (y * scale) as u32;
        if x < skin.width() && y < skin.height() {
            skin.get_pixel(x, y).0[3]
        } else {
            0
        }
    };
    // Column 47 (0-based) is the classic arm's fourth pixel.
    (20..32).all(|y| sample(47.0, y as f32) == 0)
}

/// The front-facing player model, `16*UNIT` wide and `32*UNIT` tall.
pub fn model_image(skin: &RgbaImage, slim: bool) -> Image {
    let width = 16 * UNIT;
    let height = 32 * UNIT;
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let scale = skin.width() as f32 / 64.0;
    let arm = if slim { 3u32 } else { 4u32 };
    let body_x = 4u32;

    // `(source, destination)` in 64x64 skin coordinates: base layers first, then
    // each overlay so it blends over.
    let parts: [(u32, u32, u32, u32, u32, u32); 12] = [
        // head, then hat
        (8, 8, 8, 8, body_x, 0),
        (40, 8, 8, 8, body_x, 0),
        // torso, then jacket
        (20, 20, 8, 12, body_x, 8),
        (20, 36, 8, 12, body_x, 8),
        // right arm (screen left), then sleeve
        (44, 20, arm, 12, body_x - arm, 8),
        (44, 36, arm, 12, body_x - arm, 8),
        // left arm (screen right), then sleeve
        (36, 52, arm, 12, body_x + 8, 8),
        (52, 52, arm, 12, body_x + 8, 8),
        // right leg (screen left), then trouser
        (4, 20, 4, 12, body_x, 20),
        (4, 36, 4, 12, body_x, 20),
        // left leg (screen right), then trouser
        (20, 52, 4, 12, body_x + 4, 20),
        (4, 52, 4, 12, body_x + 4, 20),
    ];

    for (sx, sy, sw, sh, dx, dy) in parts {
        blit(
            &mut buffer,
            width,
            height,
            skin,
            scale,
            sx,
            sy,
            sw,
            sh,
            dx * UNIT,
            dy * UNIT,
            sw * UNIT,
            sh * UNIT,
        );
    }
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&buffer, width, height))
}

/// The cape's front face, `10*UNIT` wide and `17*UNIT` tall, or `None` for a
/// texture that is not a 2:1, 64px-wide cape sheet.
pub fn cape_image(cape: &RgbaImage) -> Option<Image> {
    let width = cape.width();
    // A cape texture is 64x32 (or a multiple), 2:1, and its face is at (1,0).
    if width == 0 || !width.is_multiple_of(64) || width != cape.height() * 2 {
        return None;
    }
    let out_w = 10 * UNIT;
    let out_h = 17 * UNIT;
    let mut buffer = vec![0u8; (out_w * out_h * 4) as usize];
    let scale = width as f32 / 64.0;
    blit(
        &mut buffer,
        out_w,
        out_h,
        cape,
        scale,
        1,
        0,
        10,
        17,
        0,
        0,
        out_w,
        out_h,
    );
    Some(Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
        &buffer, out_w, out_h,
    )))
}

/// Nearest-neighbour blit of a source region into a destination rectangle,
/// source-over blending onto whatever is already there.
#[allow(clippy::too_many_arguments)]
fn blit(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    src: &RgbaImage,
    scale: f32,
    src_x: u32,
    src_y: u32,
    src_w: u32,
    src_h: u32,
    dst_x: u32,
    dst_y: u32,
    dst_w_box: u32,
    dst_h_box: u32,
) {
    for y in 0..dst_h_box {
        for x in 0..dst_w_box {
            let sample_x = (src_x as f32 * scale
                + (x as f32 + 0.5) * (src_w as f32 * scale / dst_w_box as f32))
                .floor() as i64;
            let sample_y = (src_y as f32 * scale
                + (y as f32 + 0.5) * (src_h as f32 * scale / dst_h_box as f32))
                .floor() as i64;
            if sample_x < 0 || sample_y < 0 {
                continue;
            }
            let (sample_x, sample_y) = (sample_x as u32, sample_y as u32);
            if sample_x >= src.width() || sample_y >= src.height() {
                continue;
            }
            let source = src.get_pixel(sample_x, sample_y).0;
            if source[3] == 0 {
                continue;
            }
            let dx = dst_x + x;
            let dy = dst_y + y;
            if dx >= dst_w || dy >= dst_h {
                continue;
            }
            let index = ((dy * dst_w + dx) * 4) as usize;
            let destination = [dst[index], dst[index + 1], dst[index + 2], dst[index + 3]];
            dst[index..index + 4].copy_from_slice(&over(source, destination));
        }
    }
}

/// Source-over compositing, the blend the overlays use.
fn over(source: [u8; 4], destination: [u8; 4]) -> [u8; 4] {
    let channel = |index: usize| source[index] as f32 / 255.0;
    let alpha_s = channel(3);
    let alpha_d = destination[3] as f32 / 255.0;
    let alpha = alpha_s + alpha_d * (1.0 - alpha_s);
    if alpha <= 0.0 {
        return [0; 4];
    }
    let mix = |index: usize| {
        let value = (channel(index) * alpha_s
            + (destination[index] as f32 / 255.0) * alpha_d * (1.0 - alpha_s))
            / alpha;
        (value * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [mix(0), mix(1), mix(2), (alpha * 255.0).round() as u8]
}
