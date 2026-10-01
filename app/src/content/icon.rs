// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The project icon cache and codec.

use super::*;

/// Fetches and decodes a card icon, on the background thread.
///
/// A local icon is a `data:` URL — what the `content` crate emits for a mod's,
/// a resource pack's or a save's own icon — and a remote project's is an
/// `https:` one. Blocking, which is what `crate::runtime::block_on` is for.
pub(crate) fn fetch_icon(url: &str) -> Option<PendingImage> {
    let bytes = crate::runtime::block_on(fetch_icon_bytes(url))?;
    decode_icon(url, bytes)
}

/// The icon's bytes, and nothing else.
///
/// Split out of [`fetch_icon`] so a caller that wants the icons of a whole page
/// at once can *await* the network and leave only the decode to a blocking
/// thread. Downloading twenty of them one after another on one thread is what
/// makes a list of remote results look dead while its images arrive.
pub(crate) async fn fetch_icon_bytes(url: &str) -> Option<Vec<u8>> {
    if url.is_empty() {
        return None;
    }
    // Already decoded for an earlier list: nothing to fetch.
    if ICONS.with(|cache| cache.borrow().contains_key(url)) {
        return None;
    }
    if let Some(data) = url.strip_prefix("data:") {
        return decode_base64(data);
    }
    let response = shared::HTTP_CLIENT.get(url).send().await.ok()?;
    Some(response.bytes().await.ok()?.to_vec())
}

/// Decodes what [`fetch_icon_bytes`] brought back. A local icon and a remote
/// project's differ only in how the bytes were obtained.
pub(crate) fn decode_icon(url: &str, bytes: Vec<u8>) -> Option<PendingImage> {
    let (width, height, rgba) = decode_to_rgba(&bytes)?;
    Some(PendingImage {
        key: url.to_string(),
        width,
        height,
        rgba,
    })
}

/// Builds an `Image` on the UI thread, memoised by key — a card is rebuilt on
/// every relayout, so without the cache a resize would re-make every icon on
/// screen.
pub(crate) fn resolve_icon(image: PendingImage) -> Option<Image> {
    resolve_image(image, &ICONS)
}

/// The card icon a save, mod, resource pack or pack falls back to — the
/// `v-else` branch every content card in the Vue carried, which pointed at
/// `Unknown_server.webp` (now `app/ui/assets/images/unknown-server.webp`). A
/// card whose icon is missing or
/// failed to decode used to come out as an empty 72x72 box here.
///
/// Decoded on first use and kept: it is the same 120x120 bitmap every time, and
/// a card is rebuilt on every relayout.
pub(crate) fn unknown_icon() -> Option<Image> {
    if let Some(image) = UNKNOWN_ICON.with(|cached| cached.borrow().clone()) {
        return Some(image);
    }
    let bytes = include_bytes!("../../ui/assets/images/unknown-server.webp");
    let (width, height, rgba) = decode_to_rgba(bytes)?;
    let image = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&rgba, width, height));
    UNKNOWN_ICON.with(|cached| *cached.borrow_mut() = Some(image.clone()));
    Some(image)
}

/// The icon at `url`, if some earlier list already fetched and decoded it.
///
/// `fetch_icon` reports a cache hit by returning nothing, so a caller that wants
/// the icon either way — the command palette redraws its results on every
/// keystroke — asks for the cache when the fetch came back empty.
pub(crate) fn cached_icon(url: &str) -> Option<Image> {
    if url.is_empty() {
        return None;
    }
    ICONS.with(|cache| cache.borrow().get(url).cloned())
}

/// One image of a detail panel's gallery strip: the bitmap plus the box it
/// takes. `.gallery-item img { height: 100%; width: auto }` sizes the box by the
/// image's own aspect ratio, and Slint cannot read an image's natural size — the
/// decoded bitmap's dimensions are only known here.
pub(crate) fn resolve_gallery_shot(image: PendingImage) -> Option<GalleryShot> {
    resolve_gallery_shot_with(image, &ICONS)
}

pub(crate) fn resolve_gallery_shot_with(
    image: PendingImage,
    cache: &'static std::thread::LocalKey<RefCell<HashMap<String, Image>>>,
) -> Option<GalleryShot> {
    let ratio = if image.height == 0 {
        1.0
    } else {
        image.width as f32 / image.height as f32
    };
    resolve_image(image, cache).map(|image| GalleryShot { image, ratio })
}

pub(crate) fn resolve_image(
    image: PendingImage,
    cache: &'static std::thread::LocalKey<RefCell<HashMap<String, Image>>>,
) -> Option<Image> {
    if let Some(cached) = cache.with(|cache| cache.borrow().get(&image.key).cloned()) {
        return Some(cached);
    }
    let built = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
        &image.rgba,
        image.width,
        image.height,
    ));
    cache.with(|cache| {
        cache.borrow_mut().insert(image.key, built.clone());
    });
    Some(built)
}

/// `image/png;base64,AAAA…` — the part of a `data:` URL after `data:`.
///
/// The `content` crate emits two paddings — `STANDARD` for a save's icon and
/// `STANDARD_NO_PAD` for a mod's or a resource pack's — so both are accepted.
pub(crate) fn decode_base64(data: &str) -> Option<Vec<u8>> {
    let (meta, payload) = data.split_once(',')?;
    if !meta.contains("base64") {
        return None;
    }
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload.trim()))
        .ok()
}

/// The decoded pixels, as the background thread produces them.
pub(crate) fn decode_to_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some((width, height, rgba.into_raw()))
}
