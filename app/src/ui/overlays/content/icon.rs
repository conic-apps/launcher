// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The project icon cache and codec.

use futures::{StreamExt, stream};

use super::*;

/// How many icons are fetched at once.
///
/// They are small images from a CDN, so the cap is about the CDN rather than the
/// app: sixteen keeps a page's worth in flight without looking like an attack.
/// The point of the cap is that they overlap at all — one at a time is what made
/// a page of twenty results look dead for twenty round-trips.
pub(crate) const ICON_FETCH_CONCURRENCY: usize = 16;

/// How many decoded images one of these maps holds.
///
/// An entry is a decoded bitmap at the source's own resolution — a Modrinth icon
/// is 128×128, so 64 kB of RGBA plus whatever the driver keeps alongside it.
/// A browsing session that pages through thousands of projects would otherwise
/// grow this without bound; five hundred is well past what a person scrolls
/// past in one sitting and around 25 MB at that per-icon size.
const DECODED_IMAGE_LIMIT: usize = 500;

/// One icon on its way to the cards: where it comes from, and every
/// (row, card id) that wants it. Folded by url because a page can show the same
/// default icon on several rows, and one download serves all of them.
pub(crate) type PendingIcon = (String, Vec<(usize, String)>);

/// Fetches the icons a freshly delivered grid is still missing and drops each one
/// in as it arrives.
///
/// Called by [`set_cards`] right after the cards go to the model, so a grid is on
/// screen before a single icon has been fetched. Every fetch is reported on its
/// own rather than in one batch at the end, because the last icon of a slow page
/// would otherwise hold back the sixty that are already there.
///
/// A card whose row has since been replaced is skipped: the row's `id` is
/// checked against the ones recorded here, which covers an instance switch,
/// another list landing in the same model, and the same list reloaded. It is more
/// precise than a generation counter for this — a card's id *is* its identity,
/// so a match means the row still wants this icon.
pub(crate) fn load_icons(ui: &App, grid: Grid, icons: Vec<PendingIcon>) {
    if icons.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let fetches = stream::iter(icons).map(|(url, rows)| {
            let weak = weak.clone();
            async move {
                let image = fetch_icon(&url);
                (rows, weak, image)
            }
        });
        fetches
            .buffer_unordered(ICON_FETCH_CONCURRENCY)
            .for_each(|(rows, weak, image)| async move {
                crate::ui::services::report::report(&weak, move |ui| {
                    set_card_icon(&ui, grid, &rows, image);
                });
            })
            .await;
    });
}

/// Puts one icon into every row that asked for it, if those rows are still the
/// cards that asked.
///
/// A failed fetch is not a special case: the cards simply end up with no icon,
/// which is what a card with no icon to begin with draws.
fn set_card_icon(ui: &App, grid: Grid, rows: &[(usize, String)], image: Option<PendingImage>) {
    // The same grid a resize is allowed to re-lay out; a different one means the
    // panel moved on and these rows belong to something else now.
    if controller().borrow().open_grid != Some(grid) {
        return;
    }
    let model = grid_model(ui, grid);
    // Resolved once and cloned per row: `resolve_icon` memoises by url, so a
    // second row carrying the same icon would get the same `Image` back anyway.
    let icon = image
        .and_then(resolve_icon)
        .or_else(unknown_icon)
        .unwrap_or_default();
    for (index, id) in rows {
        let Some(mut card) = model.row_data(*index) else {
            continue;
        };
        if card.id != *id {
            continue;
        }
        card.icon = icon.clone();
        card.icon_loading = false;
        model.set_row_data(*index, card);
    }
}

/// Which decoded-image memo a source lives in.
///
/// A mod's, a resource pack's and a save's icon is a `data:` url or an `https:`
/// one and is remembered in `ICONS`; a screenshot is a file of its own and is
/// remembered in `SCREENSHOTS`, under its path. The cards already treat the two
/// separately; the game view's preview rows draw both, so it names the difference
/// once instead of branching at every call site.
///
/// The three steps are the three moments a caller has them: [`Memo::get`] on the
/// UI thread when it already has a key, [`Memo::fetch`] on the runtime for what
/// it does not, and [`Memo::put`] back on the UI thread to build the `Image`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Memo {
    Icons,
    Screenshots,
}

impl Memo {
    /// The image at `key`, if some earlier request already decoded it.
    pub(crate) fn get(self, key: &str) -> Option<Image> {
        match self {
            Memo::Icons => cached_icon(key),
            Memo::Screenshots => SCREENSHOTS.with(|shots| shots.borrow().get(key).cloned()),
        }
    }

    /// Fetches and decodes what is at `key`, off the UI thread.
    ///
    /// Blocking, which is what `crate::support::runtime::block_on` is for and the
    /// same call [`load_icons`] makes. A screenshot is read from the instance's
    /// own directory rather than fetched, so it does not go through the http
    /// cache — that is what would reject a path outright.
    pub(crate) fn fetch(self, key: &str) -> Option<PendingImage> {
        match self {
            Memo::Icons => fetch_icon(key),
            Memo::Screenshots => {
                let bytes = std::fs::read(key).ok()?;
                let (width, height, rgba) = decode_to_rgba(&bytes)?;
                Some(PendingImage {
                    key: key.to_string(),
                    width,
                    height,
                    rgba,
                })
            }
        }
    }

    /// Makes the `Image` on the UI thread and remembers it, so the next row that
    /// wants the same source is a lookup rather than a decode.
    pub(crate) fn put(self, image: PendingImage) -> Option<Image> {
        match self {
            Memo::Icons => resolve_icon(image),
            Memo::Screenshots => resolve_image(image, &SCREENSHOTS),
        }
    }
}

/// Fetches and decodes a card icon, on the background thread.
///
/// A local icon is a `data:` URL — what the `content` crate emits for a mod's,
/// a resource pack's or a save's own icon — and a remote project's is an
/// `https:` one. Blocking, which is what `crate::support::runtime::block_on` is for.
pub(crate) fn fetch_icon(url: &str) -> Option<PendingImage> {
    let bytes = crate::support::runtime::block_on(fetch_icon_bytes(url))?;
    if let Some(image) = decode_icon(url, bytes) {
        return Some(image);
    }
    // The bytes did not decode. If they came off disk that may be the cache's
    // fault rather than the host's — a truncated write, or an entry left by a
    // version that stored something else — and re-reading it would only produce
    // the same bytes again. Ask the server once, and give up if that fails too.
    let bytes = crate::support::runtime::block_on(force_fetch_icon_bytes(url))?;
    decode_icon(url, bytes)
}

/// The icon's bytes, and nothing else.
///
/// Split out of [`fetch_icon`] so a caller that wants the icons of a whole page
/// at once can *await* the network and leave only the decode to a blocking
/// thread. Downloading twenty of them one after another on one thread is what
/// makes a list of remote results look dead while its images arrive.
///
/// A `data:` URL is decoded here rather than by [`shared::http_cache`]: it is a
/// payload the caller already holds, not something fetched, and only the caller
/// knows which base64 padding to expect. Everything else goes through the cache,
/// which is where the disk copy, the request timeout, the status check and the
/// refusal to fetch a host on the user's own network all come from.
pub(crate) async fn fetch_icon_bytes(url: &str) -> Option<Vec<u8>> {
    if url.is_empty() {
        return None;
    }
    match url.strip_prefix("data:") {
        Some(data) => decode_base64(data),
        None => shared::http_cache::fetch(url).await,
    }
}

/// [`fetch_icon_bytes`] for a `data:` URL, which there is nothing to refetch
/// for — the payload is in the URL. Only the `https:` half reaches the cache's
/// reload path, so a local icon that fails to decode just fails.
async fn force_fetch_icon_bytes(url: &str) -> Option<Vec<u8>> {
    if url.starts_with("data:") {
        return None;
    }
    shared::http_cache::force_fetch(url).await
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

/// The card icon a save, mod, resource pack or pack falls back to:
/// `app/ui/assets/images/unknown-server.webp`. A card whose icon is missing or
/// failed to decode would otherwise come out as an empty box.
///
/// Decoded on first use and kept: it is the same 120x120 bitmap every time, and
/// a card is rebuilt on every relayout.
pub(crate) fn unknown_icon() -> Option<Image> {
    if let Some(image) = UNKNOWN_ICON.with(|cached| cached.borrow().clone()) {
        return Some(image);
    }
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ui/assets/images/unknown-server.webp"
    ));
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
/// takes. The box is sized by the image's own aspect ratio, computed from the
/// decoded bitmap's dimensions.
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
        let mut cache = cache.borrow_mut();
        if cache.len() >= DECODED_IMAGE_LIMIT {
            // A flush rather than a selective eviction: a `HashMap` has no order
            // to evict from the back of, and guessing wrong would blank an icon
            // that is on screen. The disk cache is what makes throwing them away
            // cheap — the next card built for one reads a local file instead of
            // asking a CDN — so this costs a re-decode, not a round-trip.
            cache.clear();
        }
        cache.insert(image.key, built.clone());
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
