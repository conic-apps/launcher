// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The player-head image `AccountAvatar` draws, plus the 18 default skins
//! bundled in `app/ui/assets/skins`.
//!
//! The head is cropped out of the skin texture: the 8x8 face region is drawn
//! inset into the square, then the 8x8 hat layer is stretched over the whole of
//! it — both nearest-neighbour, which is what `image-rendering: pixelated`
//! shows. This reproduces that in a pixel buffer, including the source-over
//! blend where the hat is opaque.
//!
//! The square is left square here. The corner rounding happens in [`round_image`],
//! which the `AvatarMask.rounded` callback (`app/ui/globals/avatar.slint`)
//! runs as `AccountAvatar` draws, so each caller picks its own radius. It cannot
//! be Slint's `clip: true` + `border-radius`: a clipped box is clipped
//! **rectangularly** by Slint's software renderer (`i-slint-renderer-software`'s
//! `combine_clip` ignores the radius), and the winit backend falls back to that
//! renderer without a word when femtovg's OpenGL context cannot be created — a
//! VM, an RDP session, a disabled or outdated GPU driver. Slint's `Rectangle`
//! takes no `background-image` and `Image` takes no `border-radius`, so a
//! rounded *fill* cannot stand in for a rounded *image* either: the alpha is the
//! only mask a software renderer will honour.
//!
//! It lives in the app rather than in `account` because it produces a Slint
//! `Image` and reads the app's bundled assets; `filter_neoforge_version_list`
//! lives in `app/src` for the same reason.

use account::Account;
use base64::{Engine, engine::general_purpose};
use slint::{ComponentHandle, Image, SharedPixelBuffer};

use crate::slint_backend::{App, AvatarMask};

/// A skin of the bundled default set.
///
/// It carries the two names and the texture bytes together because
/// `include_bytes!` needs a literal path, so the table is the only place the
/// name and the file are known together.
#[derive(Clone, Copy)]
pub struct DefaultSkin {
    // The two names `bundled_skin_image` looks a skin up by.
    pub texture_name: &'static str,
    pub model_type: &'static str,
    texture: &'static [u8],
}

/// The slim models first, then the wide ones, each in the same order.
/// `get_default_skin` indexes into this, so the order is part of the mapping and
/// must not be sorted.
const DEFAULT_SKINS: [DefaultSkin; 18] = [
    DefaultSkin {
        texture_name: "alex",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/alex.webp"
        )),
    },
    DefaultSkin {
        texture_name: "ari",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/ari.webp"
        )),
    },
    DefaultSkin {
        texture_name: "efe",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/efe.webp"
        )),
    },
    DefaultSkin {
        texture_name: "kai",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/kai.webp"
        )),
    },
    DefaultSkin {
        texture_name: "makena",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/makena.webp"
        )),
    },
    DefaultSkin {
        texture_name: "noor",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/noor.webp"
        )),
    },
    DefaultSkin {
        texture_name: "steve",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/steve.webp"
        )),
    },
    DefaultSkin {
        texture_name: "sunny",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/sunny.webp"
        )),
    },
    DefaultSkin {
        texture_name: "zuri",
        model_type: "slim",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/slim/zuri.webp"
        )),
    },
    DefaultSkin {
        texture_name: "alex",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/alex.webp"
        )),
    },
    DefaultSkin {
        texture_name: "ari",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/ari.webp"
        )),
    },
    DefaultSkin {
        texture_name: "efe",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/efe.webp"
        )),
    },
    DefaultSkin {
        texture_name: "kai",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/kai.webp"
        )),
    },
    DefaultSkin {
        texture_name: "makena",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/makena.webp"
        )),
    },
    DefaultSkin {
        texture_name: "noor",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/noor.webp"
        )),
    },
    DefaultSkin {
        texture_name: "steve",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/steve.webp"
        )),
    },
    DefaultSkin {
        texture_name: "sunny",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/sunny.webp"
        )),
    },
    DefaultSkin {
        texture_name: "zuri",
        model_type: "wide",
        texture: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/assets/skins/wide/zuri.webp"
        )),
    },
];

impl DefaultSkin {
    /// The skin texture bytes, for callers that draw more than the head.
    pub fn texture_bytes(&self) -> &'static [u8] {
        self.texture
    }

    /// Whether this is one of the slim (3px-arm) models.
    pub fn is_slim(&self) -> bool {
        self.model_type == "slim"
    }
}

/// The default skin a UUID picks.
///
/// The UUID is hashed the way Java's `UUID.hashCode()` does — the two halves of
/// the 128 bits XORed together, then folded to an `int` — and reduced modulo the
/// 18 skins, so the index is always in range. `None` for a UUID that does not
/// parse, and the caller then leaves the placeholder up.
pub fn get_default_skin(uuid: &str) -> Option<DefaultSkin> {
    let hash = uuid_hash_code(uuid)?;
    Some(DEFAULT_SKINS[hash.rem_euclid(DEFAULT_SKINS.len() as i32) as usize])
}

/// The image `AccountAvatar` shows for a profile: its own skin when it has one,
/// the default skin of its UUID otherwise. A skin wins; the default skin is only
/// reached without one.
pub fn avatar_image(skin_url: Option<&str>, uuid: &str, size: u32) -> Option<Image> {
    let texture = match skin_url {
        Some(url) => decode_skin_url(url)?,
        None => decode(get_default_skin(uuid)?.texture)?,
    };
    Some(head(&texture, size))
}

/// The head of one of the bundled default skins, by the two names a
/// [`DefaultSkin`] carries.
///
/// The footer reaches for the wide Steve this way for its logged-out avatar;
/// every other caller goes through [`account_head`].
pub fn bundled_skin_image(texture_name: &str, model_type: &str, size: u32) -> Option<Image> {
    let skin = DEFAULT_SKINS
        .iter()
        .find(|skin| skin.texture_name == texture_name && skin.model_type == model_type)?;
    Some(head(&decode(skin.texture)?, size))
}

/// The head the footer draws for an account.
///
/// `None` is the logged-out branch, where the Steve head is dimmed with the CSS
/// `filter: grayscale(1)`, which Slint has no equivalent of — so the pixels are
/// desaturated here instead. A skin that cannot be turned into pixels (a URL the
/// crate failed to download, a UUID that does not parse) falls back to the
/// caller's placeholder disc.
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
    // `to_rgba8` already hands back a buffer of its own, so the pixels are
    // desaturated where they are.
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

/// The texture URL an account's own skin lives at, if it has one.
pub fn account_skin_url(account: &Account) -> Option<String> {
    match account {
        Account::Microsoft(account) => account.profile.skins.first().map(|skin| skin.url.clone()),
        Account::Yggdrasil(account) => account::yggdrasil::get_skin_url(&account.profile),
        Account::Offline(account) => account.skin.clone(),
    }
}

/// Decodes a `data:` URL — what the account crates store a skin as once they
/// have downloaded it.
///
/// A skin the crate could *not* download is left as the URL it came with; Slint
/// cannot fetch an image itself, so those fall back to the placeholder head.
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

/// Decodes encoded image bytes (a PNG/WebP skin or cape) into pixels.
pub(crate) fn decode(bytes: &[u8]) -> Option<image::RgbaImage> {
    image::load_from_memory(bytes)
        .ok()
        .map(|image| image.into_rgba8())
}

/// Draws the head of `skin` into a `size`x`size` buffer.
///
/// The face is inset by `round(size / 18)` on every side, then the hat is
/// stretched over the whole square and blended on top. The result is square and
/// premultiplied; [`round_image`] cuts the corners, and does that at draw time
/// rather than here so the radius is the caller's to choose.
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

/// Cuts `image` to a rounded rectangle of `radius`, the way the
/// `AvatarMask.rounded` callback asks for. It is square for the account pills'
/// heads and portrait for the capes.
///
/// `draw-size` is the width the caller draws the image at, so `radius` is
/// measured on screen rather than in the source's pixels: `AccountAvatar`
/// generates the head at its own `size` but insets it 4px as it draws, and a
/// radius in source pixels would come out proportionally small. The mask runs in
/// the source's pixel space with the radius carried into it, so the two sizes
/// agree. A radius of half the shorter side or more is the inscribed pill (a
/// circle on a square).
///
/// The pixels are premultiplied, so the mask multiplies every channel by the
/// coverage rather than only the alpha — which is what keeps the rim from
/// reading too bright, the same reason [`premultiply`] exists below.
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

/// CSS `filter: drop-shadow(0 <offset-y> <blur> <color>)` of `image`, on a canvas
/// grown by `pad` on every side.
///
/// Slint's `drop-shadow-*` follows an element's box, so a head with transparent
/// corners would carry a square shadow; this blurs the image's own alpha
/// instead. The caller draws the result behind the head at `(-pad, -pad)`, which
/// is why the canvas is grown rather than the blur being clipped at the head's
/// edge. The three box passes below stand in for the Gaussian CSS specifies.
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
        // The shadow is the colour at `blurred * alpha`, premultiplied: the
        // colour is scaled by that alpha so the black of `#00000066` stays black.
        let coverage = coverage * color_alpha;
        let pixel = [red * coverage, green * coverage, blue * coverage, coverage];
        pixels[index * 4..index * 4 + 4].copy_from_slice(&to_bytes(pixel));
    }
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, out, out))
}

/// One separable box blur of `radius` over the square `data`.
///
/// Naive on purpose: the avatars are tens of pixels, so the O(radius) window is
/// cheaper than the scratch space and bookkeeping a running sum would need.
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

/// Wires the `AvatarMask` global (`app/ui/globals/avatar.slint`) to the two pixel
/// effects above.
///
/// The callbacks are how the radius and the shadow reach the pixels: Slint cannot
/// round an `Image` or shadow its alpha itself (see the module doc), so
/// `AccountAvatar`'s `corner-radius` and the account view's CSS-like
/// `drop-shadow` are answered here.
pub fn setup(ui: &App) {
    let mask = ui.global::<AvatarMask>();
    mask.on_rounded(|image, radius, draw_size| round_image(&image, radius, draw_size));
    mask.on_drop_shadow(|image, blur, offset_y, pad, color| {
        drop_shadow(&image, blur, offset_y, pad, color)
    });
}

/// How much of the pixel at (`x`, `y`) a `width`x`height` rectangle with corners
/// of `radius` covers, from 0 to 1.
///
/// The signed distance to a rounded rectangle, ramped over one pixel so the rim
/// is antialiased. `radius` 0 is the full rectangle and `radius` half the shorter
/// side is the inscribed pill, so one function serves every corner the avatars
/// and capes round. The half pixel of slack is what makes the rim a ramp rather
/// than a hard cut, matching the rounded fill the image is drawn on.
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

/// The straight-alpha pixel the layers were composed into, made premultiplied.
///
/// `SharedPixelBuffer` is premultiplied and every renderer reads it that way
/// (`TexturePixelFormat::RgbaPremultiplied`), so this is what the channel
/// convention asks for. It is the identity on the head's opaque interior — the
/// hat layer over the face leaves nothing translucent — so it only shows at the
/// circle's rim, where a straight colour read as premultiplied comes out too
/// bright and the head would wear a halo.
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
    /// The region's top-left in skin pixels (8x8 of them).
    region: (f32, f32),
    /// The image coordinate the destination square starts at.
    origin: u32,
    side: u32,
}

impl Layer {
    /// Nearest-neighbour sample for the destination pixel (`dest_x`, `dest_y`).
    ///
    /// A destination pixel is sampled at its centre, which is what makes the
    /// mapping line up instead of being half a pixel off everywhere; the region
    /// is 8 skin pixels wide and the destination square is `self.side` wide, so
    /// the two are scaled by `scale` in between.
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

/// Java's `UUID.hashCode()`: the two halves XORed, then folded to an `int`.
///
/// Both the hyphenated and the plain form are accepted, so the separators are
/// dropped first.
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
