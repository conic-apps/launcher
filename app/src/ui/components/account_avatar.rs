// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The player-head image `AccountAvatar` draws, plus the bundled default skins.
//!
//! The head is cropped from the skin texture and rounded in pixels: Slint's
//! software renderer clips a rounded box rectangularly, and a rounded fill cannot
//! mask an image, so the alpha is the only mask it honours.

use account::Account;
use base64::{Engine, engine::general_purpose};
use conic_ui::assets::skins;
use slint::{ComponentHandle, Image, SharedPixelBuffer};

use crate::slint_backend::{App, AvatarMask};

#[derive(Clone, Copy)]
pub struct DefaultSkin {
    pub texture_name: &'static str,
    pub model_type: &'static str,
    texture: &'static [u8],
}

/// Slim models first, then wide, each in the same order; `get_default_skin`
/// indexes into this, so it must not be sorted.
const DEFAULT_SKINS: [DefaultSkin; 18] = [
    DefaultSkin {
        texture_name: "alex",
        model_type: "slim",
        texture: skins::SLIM_ALEX,
    },
    DefaultSkin {
        texture_name: "ari",
        model_type: "slim",
        texture: skins::SLIM_ARI,
    },
    DefaultSkin {
        texture_name: "efe",
        model_type: "slim",
        texture: skins::SLIM_EFE,
    },
    DefaultSkin {
        texture_name: "kai",
        model_type: "slim",
        texture: skins::SLIM_KAI,
    },
    DefaultSkin {
        texture_name: "makena",
        model_type: "slim",
        texture: skins::SLIM_MAKENA,
    },
    DefaultSkin {
        texture_name: "noor",
        model_type: "slim",
        texture: skins::SLIM_NOOR,
    },
    DefaultSkin {
        texture_name: "steve",
        model_type: "slim",
        texture: skins::SLIM_STEVE,
    },
    DefaultSkin {
        texture_name: "sunny",
        model_type: "slim",
        texture: skins::SLIM_SUNNY,
    },
    DefaultSkin {
        texture_name: "zuri",
        model_type: "slim",
        texture: skins::SLIM_ZURI,
    },
    DefaultSkin {
        texture_name: "alex",
        model_type: "wide",
        texture: skins::WIDE_ALEX,
    },
    DefaultSkin {
        texture_name: "ari",
        model_type: "wide",
        texture: skins::WIDE_ARI,
    },
    DefaultSkin {
        texture_name: "efe",
        model_type: "wide",
        texture: skins::WIDE_EFE,
    },
    DefaultSkin {
        texture_name: "kai",
        model_type: "wide",
        texture: skins::WIDE_KAI,
    },
    DefaultSkin {
        texture_name: "makena",
        model_type: "wide",
        texture: skins::WIDE_MAKENA,
    },
    DefaultSkin {
        texture_name: "noor",
        model_type: "wide",
        texture: skins::WIDE_NOOR,
    },
    DefaultSkin {
        texture_name: "steve",
        model_type: "wide",
        texture: skins::WIDE_STEVE,
    },
    DefaultSkin {
        texture_name: "sunny",
        model_type: "wide",
        texture: skins::WIDE_SUNNY,
    },
    DefaultSkin {
        texture_name: "zuri",
        model_type: "wide",
        texture: skins::WIDE_ZURI,
    },
];

impl DefaultSkin {
    pub fn texture_bytes(&self) -> &'static [u8] {
        self.texture
    }

    pub fn is_slim(&self) -> bool {
        self.model_type == "slim"
    }
}

/// The default skin a UUID picks, hashed the way Java's `UUID.hashCode()` does.
/// `None` for a UUID that does not parse.
pub fn get_default_skin(uuid: &str) -> Option<DefaultSkin> {
    let hash = uuid_hash_code(uuid)?;
    Some(DEFAULT_SKINS[hash.rem_euclid(DEFAULT_SKINS.len() as i32) as usize])
}

/// The image `AccountAvatar` shows for a profile: its own skin when it has one,
/// otherwise the UUID's default.
pub fn avatar_image(skin_url: Option<&str>, uuid: &str, size: u32) -> Option<Image> {
    let texture = match skin_url {
        Some(url) => decode_skin_url(url)?,
        None => decode(get_default_skin(uuid)?.texture)?,
    };
    Some(head(&texture, size))
}

/// The head of a bundled default skin, by the two names a [`DefaultSkin`]
/// carries.
pub fn bundled_skin_image(texture_name: &str, model_type: &str, size: u32) -> Option<Image> {
    let skin = DEFAULT_SKINS
        .iter()
        .find(|skin| skin.texture_name == texture_name && skin.model_type == model_type)?;
    Some(head(&decode(skin.texture)?, size))
}

/// The head the footer draws; `None` is logged out, where the Steve head is
/// desaturated because Slint has no `grayscale` filter.
pub fn account_head(account: Option<&Account>, size: u32) -> Option<Image> {
    let Some(account) = account else {
        return Some(grayscale(bundled_skin_image("steve", "wide", size)?));
    };
    avatar_image(
        account_skin_url(account).as_deref(),
        &account.get_profile_uuid(),
        size,
    )
}

/// `filter: grayscale(1)`, in the CSS filter's own luminance weights.
fn grayscale(image: Image) -> Image {
    let Some(mut buffer) = image.to_rgba8() else {
        return image;
    };
    for pixel in buffer.make_mut_slice().iter_mut() {
        let luminance =
            (0.2126 * pixel.r as f32 + 0.7152 * pixel.g as f32 + 0.0722 * pixel.b as f32).round()
                as u8;
        pixel.r = luminance;
        pixel.g = luminance;
        pixel.b = luminance;
    }
    Image::from_rgba8(buffer)
}

pub fn account_skin_url(account: &Account) -> Option<String> {
    match account {
        Account::Microsoft(account) => account.profile.skins.first().map(|skin| skin.url.clone()),
        Account::Yggdrasil(account) => account::yggdrasil::get_skin_url(&account.profile),
        Account::Offline(account) => account.skin.clone(),
    }
}

/// Decodes a `data:` URL, which is how a downloaded skin is stored. A skin that
/// could not be downloaded stays a URL, and Slint cannot fetch those.
pub(crate) fn decode_skin_url(url: &str) -> Option<image::RgbaImage> {
    if !url.starts_with("data:") {
        return None;
    }
    let (_, data) = url.split_once(',')?;
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(data)
        .or_else(|_| general_purpose::STANDARD.decode(data))
        .ok()?;
    decode(&bytes)
}

pub(crate) fn decode(bytes: &[u8]) -> Option<image::RgbaImage> {
    image::load_from_memory(bytes)
        .ok()
        .map(|image| image.into_rgba8())
}

/// Draws the head of `skin` into a `size`x`size` buffer, square and
/// premultiplied; [`round_image`] cuts the corners at draw time so the radius is
/// the caller's to choose.
fn head(skin: &image::RgbaImage, size: u32) -> Image {
    let size = size.max(1);
    // An HD skin crops the same face: the texture is scaled from its 64px width.
    let scale = skin.width() as f32 / 64.0;
    let inset = ((size as f32) / 18.0).round() as u32;
    let face_side = size.saturating_sub(inset.saturating_mul(2)).max(1);

    let face = Layer {
        region: (8.0, 8.0),
        origin: inset,
        side: face_side,
    };
    let hat = Layer {
        region: (40.0, 8.0),
        origin: 0,
        side: size,
    };

    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let mut pixel = [0f32; 4];
            if x >= inset
                && x < size - inset
                && y >= inset
                && y < size - inset
                && let Some(sample) = face.sample(skin, scale, x, y)
            {
                pixel = to_float(sample);
            }
            if let Some(sample) = hat.sample(skin, scale, x, y) {
                pixel = over(to_float(sample), pixel);
            }
            let index = ((y * size + x) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&to_bytes(premultiply(pixel)));
        }
    }
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, size, size))
}

/// Cuts `image` to a rounded rectangle of `radius`. `draw_size` is the width it
/// is drawn at, so the radius is measured on screen rather than in source pixels.
/// The premultiplied pixels are scaled in every channel, not just alpha, so the
/// rim does not read too bright.
pub fn round_image(image: &Image, radius: f32, draw_size: f32) -> Image {
    let Some(mut buffer) = image.to_rgba8() else {
        return image.clone();
    };
    let width = buffer.width();
    let height = buffer.height();
    if width == 0 || height == 0 || draw_size <= 0.0 {
        return image.clone();
    }
    let radius = radius * width as f32 / draw_size;
    for (index, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = index as u32 % width;
        let y = index as u32 / width;
        let coverage = rounded_rect_coverage(width, height, radius, x, y);
        if coverage >= 1.0 {
            continue;
        }
        let channel = |value: u8| (value as f32 * coverage).round().clamp(0.0, 255.0) as u8;
        pixel.r = channel(pixel.r);
        pixel.g = channel(pixel.g);
        pixel.b = channel(pixel.b);
        pixel.a = channel(pixel.a);
    }
    Image::from_rgba8(buffer)
}

/// CSS `filter: drop-shadow(0 <offset-y> <blur> <color>)` of `image`, on a
/// canvas grown by `pad` on every side. Slint's `drop-shadow-*` follows an
/// element's box, so this blurs the image's own alpha instead.
pub fn drop_shadow(
    image: &Image,
    blur: f32,
    offset_y: f32,
    pad: f32,
    color: slint::Color,
) -> Image {
    let Some(buffer) = image.to_rgba8() else {
        return Image::default();
    };
    let size = buffer.width();
    if size == 0 || buffer.height() != size {
        return Image::default();
    }
    let pad = pad.max(0.0).round() as u32;
    let out = size + pad * 2;

    let mut alpha = vec![0f32; (out * out) as usize];
    let shift = offset_y.round() as i32;
    for y in 0..size {
        for x in 0..size {
            let pixel = buffer.as_slice()[(y * size + x) as usize];
            let dest_y = y as i32 + pad as i32 + shift;
            if dest_y < 0 || dest_y >= out as i32 {
                continue;
            }
            alpha[((dest_y as u32) * out + x + pad) as usize] = pixel.a as f32 / 255.0;
        }
    }
    let radius = (blur / 2.0).round().max(0.0) as i32;
    for _ in 0..3 {
        box_blur(&mut alpha, out, out, radius);
    }

    let (red, green, blue) = (
        color.red() as f32 / 255.0,
        color.green() as f32 / 255.0,
        color.blue() as f32 / 255.0,
    );
    let color_alpha = color.alpha() as f32 / 255.0;
    let mut pixels = vec![0u8; (out * out * 4) as usize];
    for (index, coverage) in alpha.iter().enumerate() {
        let coverage = coverage * color_alpha;
        let pixel = [red * coverage, green * coverage, blue * coverage, coverage];
        pixels[index * 4..index * 4 + 4].copy_from_slice(&to_bytes(pixel));
    }
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, out, out))
}

/// One separable box blur of `radius` over the square `data`. Naive on purpose:
/// the avatars are tens of pixels, so the O(radius) window is cheaper than a
/// running sum.
fn box_blur(data: &mut [f32], width: u32, height: u32, radius: i32) {
    if radius <= 0 {
        return;
    }
    let mut scratch = vec![0f32; data.len()];
    for y in 0..height {
        let row = (y * width) as usize;
        for x in 0..width {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dx in -radius..=radius {
                let sample = x as i32 + dx;
                if sample >= 0 && sample < width as i32 {
                    sum += data[row + sample as usize];
                    count += 1.0;
                }
            }
            scratch[row + x as usize] = sum / count;
        }
    }
    for x in 0..width {
        for y in 0..height {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dy in -radius..=radius {
                let sample = y as i32 + dy;
                if sample >= 0 && sample < height as i32 {
                    sum += scratch[(sample as u32 * width + x) as usize];
                    count += 1.0;
                }
            }
            data[(y * width + x) as usize] = sum / count;
        }
    }
}

/// Wires the `AvatarMask` global to the pixel effects above; Slint cannot round
/// an `Image` or shadow its alpha itself.
pub fn setup(ui: &App) {
    let mask = ui.global::<AvatarMask>();
    mask.on_rounded(|image, radius, draw_size| round_image(&image, radius, draw_size));
    mask.on_drop_shadow(|image, blur, offset_y, pad, color| {
        drop_shadow(&image, blur, offset_y, pad, color)
    });
}

/// How much of the pixel at (`x`, `y`) a rounded rectangle covers: a signed
/// distance ramped over one pixel so the rim is antialiased.
fn rounded_rect_coverage(width: u32, height: u32, radius: f32, x: u32, y: u32) -> f32 {
    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;
    let radius = radius.clamp(0.0, half_w.min(half_h));
    let qx = (x as f32 + 0.5 - half_w).abs() - (half_w - radius);
    let qy = (y as f32 + 0.5 - half_h).abs() - (half_h - radius);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    let inside = qx.max(qy).min(0.0);
    (0.5 - (outside + inside - radius)).clamp(0.0, 1.0)
}

/// Makes the straight-alpha composed pixel premultiplied. `SharedPixelBuffer`
/// is read as premultiplied by every renderer, and a straight colour at the rim
/// would come out too bright and leave a halo.
fn premultiply(pixel: [f32; 4]) -> [f32; 4] {
    [
        pixel[0] * pixel[3],
        pixel[1] * pixel[3],
        pixel[2] * pixel[3],
        pixel[3],
    ]
}

/// One draw layer: the source region in skin pixels, and where the destination
/// square sits in the image.
struct Layer {
    region: (f32, f32),
    origin: u32,
    side: u32,
}

impl Layer {
    fn sample(
        &self,
        skin: &image::RgbaImage,
        scale: f32,
        dest_x: u32,
        dest_y: u32,
    ) -> Option<[u8; 4]> {
        let local =
            |dest: u32| (dest as f32 + 0.5 - self.origin as f32) * (8.0 * scale) / self.side as f32;
        let source_x = (self.region.0 * scale + local(dest_x)).floor();
        let source_y = (self.region.1 * scale + local(dest_y)).floor();
        if source_x < 0.0 || source_y < 0.0 {
            return None;
        }
        let (source_x, source_y) = (source_x as u32, source_y as u32);
        (source_x < skin.width() && source_y < skin.height())
            .then(|| skin.get_pixel(source_x, source_y).0)
    }
}

fn to_float(pixel: [u8; 4]) -> [f32; 4] {
    pixel.map(|channel| channel as f32 / 255.0)
}

fn to_bytes(pixel: [f32; 4]) -> [u8; 4] {
    pixel.map(|channel| (channel * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Source-over compositing, the blend used for the hat layer.
fn over(source: [f32; 4], destination: [f32; 4]) -> [f32; 4] {
    let alpha = source[3] + destination[3] * (1.0 - source[3]);
    if alpha <= 0.0 {
        return [0.0; 4];
    }
    let channel = |index: usize| {
        (source[index] * source[3] + destination[index] * destination[3] * (1.0 - source[3]))
            / alpha
    };
    [channel(0), channel(1), channel(2), alpha]
}

/// Java's `UUID.hashCode()`, accepting both the hyphenated and plain forms.
fn uuid_hash_code(uuid: &str) -> Option<i32> {
    let hex: String = uuid.chars().filter(|character| *character != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let most_significant = u64::from_str_radix(&hex[..16], 16).ok()?;
    let least_significant = u64::from_str_radix(&hex[16..], 16).ok()?;
    let hilo = most_significant ^ least_significant;
    Some(((hilo >> 32) as u32 ^ hilo as u32) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opaque everywhere, so every drawn pixel is opaque and what comes back
    /// out of a mask is the mask alone.
    fn opaque_skin() -> image::RgbaImage {
        image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]))
    }

    #[test]
    fn head_is_a_premultiplied_square() {
        const SIZE: u32 = 56;
        let buffer = head(&opaque_skin(), SIZE)
            .to_rgba8()
            .expect("the head is an rgba8 buffer");

        // No corner is cut here: the round is `round_image`'s job.
        assert!(buffer.as_slice().iter().all(|pixel| pixel.a == 255));
        // The skin is opaque red, so a premultiplied pixel stays fully red.
        assert!(buffer.as_slice().iter().all(|pixel| pixel.r == 255));
    }

    #[test]
    fn round_image_is_an_antialiased_circle() {
        const SIZE: u32 = 56;
        let square = head(&opaque_skin(), SIZE);
        let buffer = round_image(&square, 10000.0, SIZE as f32)
            .to_rgba8()
            .expect("the head is an rgba8 buffer");
        let at = |x: u32, y: u32| buffer.as_slice()[(y * SIZE + x) as usize];

        assert_eq!(at(SIZE / 2, SIZE / 2).a, 255, "the centre is inside");
        assert_eq!(at(0, 0).a, 0, "the corner is outside");

        let rim = (0..SIZE * SIZE)
            .map(|index| buffer.as_slice()[index as usize])
            .filter(|pixel| pixel.a > 0 && pixel.a < 255)
            .count();
        assert!(rim > 0, "the rim is a ramp rather than a hard cut");
        // The skin is opaque red, so a premultiplied pixel's red channel is its
        // alpha — which is what keeps the rim from reading too bright.
        assert!(
            (0..SIZE * SIZE).all(|index| {
                let pixel = buffer.as_slice()[index as usize];
                pixel.r == pixel.a
            }),
            "the buffer is premultiplied"
        );
    }

    #[test]
    fn round_image_keeps_the_corners_square_at_zero() {
        const SIZE: u32 = 56;
        let square = head(&opaque_skin(), SIZE);
        let buffer = round_image(&square, 0.0, SIZE as f32)
            .to_rgba8()
            .expect("the head is an rgba8 buffer");

        assert!(
            buffer.as_slice().iter().all(|pixel| pixel.a == 255),
            "a zero radius leaves every pixel filled"
        );
    }

    #[test]
    fn round_image_scales_the_radius_to_the_draw_size() {
        // A radius of half the draw size is a circle regardless of the source's
        // pixel count, because the draw size is what the user sees.
        let square = head(&opaque_skin(), 56);
        let buffer = round_image(&square, 28.0, 56.0)
            .to_rgba8()
            .expect("the head is an rgba8 buffer");
        assert_eq!(buffer.as_slice()[0].a, 0, "the corner is outside");
    }

    #[test]
    fn round_image_rounds_a_portrait_cape() {
        // A 40x68 cape: the corners are cut on the short axis and the body stays
        // filled, which the square-only head mask could not do.
        const WIDTH: u32 = 40;
        const HEIGHT: u32 = 68;
        // Opaque white, so every byte is 255 and the alpha is the mask alone.
        let pixels = vec![255u8; (WIDTH * HEIGHT * 4) as usize];
        let cape = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, WIDTH, HEIGHT));
        let rounded = round_image(&cape, 6.0, WIDTH as f32)
            .to_rgba8()
            .expect("the cape is an rgba8 buffer");
        let at = |x: u32, y: u32| rounded.as_slice()[(y * WIDTH + x) as usize];

        assert_eq!(at(0, 0).a, 0, "the corner is cut");
        assert_eq!(at(WIDTH / 2, HEIGHT / 2).a, 255, "the body is filled");
        assert_eq!(at(0, HEIGHT / 2).a, 255, "the long edge is filled");
    }

    #[test]
    fn rounded_rect_coverage_is_inscribed_and_symmetric() {
        // A radius of half the side is the circle: the mid-side pixel's centre
        // sits on the rim, so it is short of full, which is what an antialiased
        // rounded fill does at the very same edge.
        let circle = 28.0;
        let edge = rounded_rect_coverage(56, 56, circle, 0, 28);
        assert!(
            edge > 0.9 && edge < 1.0,
            "the mid-side pixel is on the rim, got {edge}"
        );
        assert_eq!(
            edge,
            rounded_rect_coverage(56, 56, circle, 55, 28),
            "the rim is symmetric"
        );
        assert_eq!(
            rounded_rect_coverage(56, 56, circle, 28, 28),
            1.0,
            "the centre is solid"
        );
        assert_eq!(
            rounded_rect_coverage(56, 56, circle, 0, 0),
            0.0,
            "the corner is empty"
        );

        // A zero radius is the whole rectangle, corners included.
        assert_eq!(rounded_rect_coverage(56, 56, 0.0, 0, 0), 1.0, "the square");
        assert_eq!(rounded_rect_coverage(56, 56, 0.0, 0, 28), 1.0, "its edge");
        assert_eq!(rounded_rect_coverage(1, 1, 0.0, 0, 0), 1.0, "a 1px head");
        // The radius clamps to the shorter side, so a portrait cape still fills
        // its long edge.
        assert_eq!(
            rounded_rect_coverage(40, 68, 100.0, 0, 34),
            1.0,
            "the cape's long edge is filled at the clamp"
        );
    }

    #[test]
    fn drop_shadow_follows_the_alpha_not_the_box() {
        const SIZE: u32 = 16;
        const PAD: u32 = 8;
        // An opaque 8x8 centre in a transparent 16x16 square, so the box and the
        // silhouette disagree at every corner.
        let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
        for y in 4..12 {
            for x in 4..12 {
                let index = ((y * SIZE + x) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
        let image = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, SIZE, SIZE));
        let shadow = drop_shadow(
            &image,
            2.0,
            2.0,
            PAD as f32,
            slint::Color::from_argb_u8(0x80, 0, 0, 0),
        )
        .to_rgba8()
        .expect("the shadow is an rgba8 buffer");
        let out = SIZE + PAD * 2;
        let at = |x: u32, y: u32| shadow.as_slice()[(y * out + x) as usize];

        assert_eq!(at(0, 0).a, 0, "the head's transparent corner casts nothing");
        assert!(
            at(out / 2, out / 2 + 2).a > 0,
            "the silhouette casts a shadow, offset down"
        );
        // Premultiplied black: every colour channel is zero, and the alpha is the
        // only thing the shadow carries.
        assert!(shadow.as_slice().iter().all(|pixel| pixel.r == 0));
        assert!(shadow.as_slice().iter().all(|pixel| pixel.a <= 0x80));
    }
}
