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
//! The square is then cut down to a circle, here rather than in `.slint`. A
//! `clip: true` box with a `border-radius` is clipped **rectangularly** by
//! Slint's software renderer (`i-slint-renderer-software`'s `combine_clip`
//! ignores the radius), and the winit backend falls back to that renderer
//! without a word when femtovg's OpenGL context cannot be created — a VM, an
//! RDP session, a disabled or outdated GPU driver. Slint's `Rectangle` takes
//! no `background-image`, so a rounded *fill* cannot stand in for a rounded
//! *image* either: the alpha is the only mask a software renderer will honour.
//!
//! It lives in the app rather than in `account` because it produces a Slint
//! `Image` and reads the app's bundled assets; `filter_neoforge_version_list`
//! lives in `app/src` for the same reason.

use account::Account;
use base64::{Engine, engine::general_purpose};
use slint::{Image, SharedPixelBuffer};

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
fn decode_skin_url(url: &str) -> Option<image::RgbaImage> {
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

fn decode(bytes: &[u8]) -> Option<image::RgbaImage> {
    image::load_from_memory(bytes)
        .ok()
        .map(|image| image.into_rgba8())
}

/// Draws the head of `skin` into a `size`x`size` buffer.
///
/// The face is inset by `round(size / 18)` on every side, then the hat is
/// stretched over the whole square and blended on top, and the square is cut
/// down to the circle the avatar is framed in (see the module doc for why that
/// has to happen in the pixels).
fn head(skin: &image::RgbaImage, size: u32) -> Image {
    let size = size.max(1);
    // An HD skin crops the same face: the texture is scaled from its 64px width.
    let scale = skin.width() as f32 / 64.0;
    let inset = ((size as f32) / 18.0).round() as u32;
    let face_side = size.saturating_sub(inset.saturating_mul(2)).max(1);

    // The face first, then the hat over it.
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
            // The circle, before the pixel leaves for the buffer: coverage
            // belongs to the alpha, and the premultiply below carries it into
            // the colour with it.
            pixel[3] *= circle_coverage(size, x, y);
            let index = ((y * size + x) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&to_bytes(premultiply(pixel)));
        }
    }
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, size, size))
}

/// How much of the pixel at (`x`, `y`) the circle inscribed in a `size`x`size`
/// square covers, from 0 to 1.
///
/// Inscribed rather than outset, so the head touches the edge of its square
/// mid-side and meets the ring Slint draws around it without a seam. The half
/// pixel of slack is what makes the rim a one pixel ramp instead of a hard cut,
/// matching the antialiasing of the rounded fill it is drawn on top of.
fn circle_coverage(size: u32, x: u32, y: u32) -> f32 {
    let radius = size as f32 / 2.0;
    let center = size as f32 / 2.0;
    let dx = x as f32 + 0.5 - center;
    let dy = y as f32 + 0.5 - center;
    (radius - dx.hypot(dy) + 0.5).clamp(0.0, 1.0)
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
    /// The destination square's side.
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
    /// out is the circle mask alone.
    fn opaque_skin() -> image::RgbaImage {
        image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255]))
    }

    #[test]
    fn head_is_an_antialiased_circle() {
        const SIZE: u32 = 56;
        let buffer = head(&opaque_skin(), SIZE)
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
    fn circle_coverage_is_inscribed_and_symmetric() {
        // The outermost pixel of an edge sits on the ramp, so it is short of
        // full: a pixel whose centre is on the circle is half covered, which is
        // what an antialiased rounded fill does at the very same edge.
        let edge = circle_coverage(56, 0, 28);
        assert!(
            edge > 0.9 && edge < 1.0,
            "the mid-side pixel is on the rim, got {edge}"
        );
        assert_eq!(edge, circle_coverage(56, 55, 28), "the rim is symmetric");
        assert_eq!(circle_coverage(56, 28, 28), 1.0, "the centre is solid");
        assert_eq!(circle_coverage(56, 0, 0), 0.0, "the corner is empty");
        assert_eq!(circle_coverage(1, 0, 0), 1.0, "a 1px head is solid");
    }
}
