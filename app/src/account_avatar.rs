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
        texture: include_bytes!("../ui/assets/skins/slim/alex.webp"),
    },
    DefaultSkin {
        texture_name: "ari",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/ari.webp"),
    },
    DefaultSkin {
        texture_name: "efe",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/efe.webp"),
    },
    DefaultSkin {
        texture_name: "kai",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/kai.webp"),
    },
    DefaultSkin {
        texture_name: "makena",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/makena.webp"),
    },
    DefaultSkin {
        texture_name: "noor",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/noor.webp"),
    },
    DefaultSkin {
        texture_name: "steve",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/steve.webp"),
    },
    DefaultSkin {
        texture_name: "sunny",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/sunny.webp"),
    },
    DefaultSkin {
        texture_name: "zuri",
        model_type: "slim",
        texture: include_bytes!("../ui/assets/skins/slim/zuri.webp"),
    },
    DefaultSkin {
        texture_name: "alex",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/alex.webp"),
    },
    DefaultSkin {
        texture_name: "ari",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/ari.webp"),
    },
    DefaultSkin {
        texture_name: "efe",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/efe.webp"),
    },
    DefaultSkin {
        texture_name: "kai",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/kai.webp"),
    },
    DefaultSkin {
        texture_name: "makena",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/makena.webp"),
    },
    DefaultSkin {
        texture_name: "noor",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/noor.webp"),
    },
    DefaultSkin {
        texture_name: "steve",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/steve.webp"),
    },
    DefaultSkin {
        texture_name: "sunny",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/sunny.webp"),
    },
    DefaultSkin {
        texture_name: "zuri",
        model_type: "wide",
        texture: include_bytes!("../ui/assets/skins/wide/zuri.webp"),
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
/// stretched over the whole square and blended on top.
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
            let index = ((y * size + x) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&to_bytes(pixel));
        }
    }
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&pixels, size, size))
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
