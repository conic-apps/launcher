// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher-content client: Mojang's news feed and the Java Edition
//! changelogs.
//!
//! Both come from the same static host the official Minecraft launcher reads,
//! `launchercontent.mojang.com`, and both are plain JSON documents with no
//! authentication and no query parameters:
//!
//!   * `/v2/news.json` — the news feed: 100 entries across every edition
//!     (Java, Bedrock, Dungeons, Legends), each with a title, a category, a
//!     date, a one-line blurb, a 772×350 banner image and a `readMoreLink` to
//!     the article on `minecraft.net`. `newsType` carries the platform tags.
//!   * `/v2/javaPatchNotes.json` — the Java Edition changelog index, every
//!     snapshot and release since 2018, each with a 540×540 (or older 240×240)
//!     square image and a `contentPath` pointing at a second document holding
//!     the full HTML body.
//!
//! Two things the API does not do, and this crate therefore does not either:
//!
//!   * **It does not localize.** `?lang=…` and `Accept-Language` are both
//!     ignored — the same bytes come back whatever the client says — and the
//!     `needsTranslation` flag says only that the official launcher has its own
//!     translation pipeline behind a private endpoint. The feed text is
//!     English, always.
//!   * **It does not page.** Each document is a fixed latest-100 / all-415
//!     list. There is no `offset`, and `?page=` returns the same body.
//!
//! The crate is UI-neutral: it yields plain data, and the app turns it into
//! models. Sorting is done here (newest first) because that is a property of
//! the feed rather than of a view.

pub mod error;

use error::*;
use serde::Deserialize;
use shared::HTTP_CLIENT;

/// The host both documents and every image are served from.
const BASE_URL: &str = "https://launchercontent.mojang.com";

/// One entry of the news feed.
///
/// `image_url` and `read_more` are already absolute — the feed stores the image
/// as a rooted path (`/v2/images/…`) and the article as a full URL.
#[derive(Debug, Clone)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    /// The edition the entry belongs to as the feed spells it ("Minecraft: Java
    /// Edition", "Minecraft for Windows", …). Shown as the card's category.
    pub category: String,
    /// The platform tags — "Java", "Bedrock", "Dungeons", "Legends",
    /// "DungeonsII", "News page". Kept so a caller can narrow the feed without
    /// re-parsing it.
    pub news_type: Vec<String>,
    /// The feed's `YYYY-MM-DD`.
    pub date: String,
    /// `date` split out, because the year and month filters are built from it.
    pub year: u16,
    pub month: u8,
    /// The one-line blurb.
    pub text: String,
    /// The 772×350 wide banner, absolute.
    pub image_url: String,
    /// The banner's own dimensions, for the card to cut its box from. The feed
    /// carries them; a document that did not would fall back to 772×350.
    pub image_width: u32,
    pub image_height: u32,
    /// The article on `minecraft.net`, absolute.
    pub read_more: String,
}

/// Whether a changelog entry is a stable release or a development snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogKind {
    Release,
    Snapshot,
}

impl ChangelogKind {
    /// The machine-readable key, so a view can resolve a translated label
    /// without having to know the Rust enum.
    pub fn key(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Snapshot => "snapshot",
        }
    }

    /// The feed writes `"release"` and `"snapshot"`; anything else — the
    /// document is generated and could grow a third kind — is treated as a
    /// snapshot, which is the majority and the least surprising default.
    fn from_raw(raw: &str) -> Self {
        if raw.eq_ignore_ascii_case("release") {
            Self::Release
        } else {
            Self::Snapshot
        }
    }
}

/// One entry of the Java changelog index.
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub kind: ChangelogKind,
    /// The ISO-8601 timestamp the feed writes.
    pub date: String,
    /// `date` split out, for the same reason as [`NewsItem`].
    pub year: u16,
    pub month: u8,
    /// The square (540×540, or 240×240 for the oldest) image, absolute.
    pub image_url: String,
    /// The image's own dimensions. The changelog index carries none, so these
    /// are 540×540 — the shape every entry of it has — unless a document grows
    /// them.
    pub image_width: u32,
    pub image_height: u32,
    /// The short blurb.
    pub short_text: String,
    /// Where the full body lives, relative to `/v2/` — e.g.
    /// `javaPatchNotes/26-4-snapshot-2.json`.
    pub content_path: String,
}

/// The raw shape of one news entry, as it is on the wire.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNewsItem {
    id: String,
    title: String,
    category: String,
    date: String,
    text: String,
    news_page_image: RawImage,
    read_more_link: String,
    #[serde(default)]
    news_type: Vec<String>,
}

/// The raw shape of one changelog-index entry.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChangelogEntry {
    id: String,
    title: String,
    version: String,
    #[serde(rename = "type")]
    kind: String,
    date: String,
    image: RawImage,
    short_text: String,
    content_path: String,
}

/// One image reference. The changelog index prints no dimensions, so the field
/// is optional.
#[derive(Debug, Deserialize)]
struct RawImage {
    url: String,
    #[serde(default)]
    dimensions: Option<Dimensions>,
}

#[derive(Debug, Deserialize)]
struct Dimensions {
    width: u32,
    height: u32,
}

/// The feed's own dimensions, or the shape the source always is.
fn dimensions(image: &RawImage, fallback: (u32, u32)) -> (u32, u32) {
    image
        .dimensions
        .as_ref()
        .map(|size| (size.width, size.height))
        .unwrap_or(fallback)
}

/// The banner the news feed always returns, and the fallback when a document
/// omits its dimensions.
const NEWS_IMAGE: (u32, u32) = (772, 350);
/// Every changelog image is square; older entries are 240×240, newer 540×540,
/// and the card only reads the ratio.
const CHANGELOG_IMAGE: (u32, u32) = (540, 540);

#[derive(Debug, Deserialize)]
struct Feed<T> {
    entries: Vec<T>,
}

/// The full HTML body of one changelog.
#[derive(Debug, Deserialize)]
struct RawChangelogBody {
    body: String,
}

/// Every image and article URL in the feeds is either absolute or rooted; both
/// forms land here so a caller never has to think about which it is.
fn absolute_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("{BASE_URL}{url}")
    }
}

/// `YYYY-MM-DD` or an ISO timestamp into `(year, month)`.
///
/// Deliberately a split rather than a date parser: both feeds put the year and
/// month first and zero-padded, so this cannot fail or drift with a locale, and
/// the crate needs no calendar dependency.
fn year_month(date: &str) -> (u16, u8) {
    let mut parts = date.split(['-', 'T']);
    let year = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let month = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    (year, month)
}

/// The latest 100 news entries, newest first.
pub async fn fetch_news() -> Result<Vec<NewsItem>> {
    let url = format!("{BASE_URL}/v2/news.json");
    let feed: Feed<RawNewsItem> = HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut items: Vec<NewsItem> = feed
        .entries
        .into_iter()
        .map(|raw| {
            let (year, month) = year_month(&raw.date);
            let (image_width, image_height) = dimensions(&raw.news_page_image, NEWS_IMAGE);
            NewsItem {
                id: raw.id,
                title: raw.title,
                category: raw.category,
                news_type: raw.news_type,
                date: raw.date,
                year,
                month,
                text: raw.text,
                image_url: absolute_url(&raw.news_page_image.url),
                image_width,
                image_height,
                read_more: absolute_url(&raw.read_more_link),
            }
        })
        .collect();
    // Newest first. The feeds are close to sorted, but relying on that would
    // put a re-ordered document on screen in the wrong order.
    items.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(items)
}

/// The Java Edition changelog index — every snapshot and release since 2018,
/// newest first.
pub async fn fetch_changelogs() -> Result<Vec<ChangelogEntry>> {
    let url = format!("{BASE_URL}/v2/javaPatchNotes.json");
    let feed: Feed<RawChangelogEntry> = HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut items: Vec<ChangelogEntry> = feed
        .entries
        .into_iter()
        .map(|raw| {
            let (year, month) = year_month(&raw.date);
            let (image_width, image_height) = dimensions(&raw.image, CHANGELOG_IMAGE);
            ChangelogEntry {
                id: raw.id,
                title: raw.title,
                version: raw.version,
                kind: ChangelogKind::from_raw(&raw.kind),
                date: raw.date,
                year,
                month,
                image_url: absolute_url(&raw.image.url),
                image_width,
                image_height,
                short_text: raw.short_text,
                content_path: raw.content_path,
            }
        })
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(items)
}

/// The full HTML of one changelog, read from the `contentPath` its index entry
/// carries.
pub async fn fetch_changelog_body(content_path: &str) -> Result<String> {
    let url = format!("{BASE_URL}/v2/{content_path}");
    let body: RawChangelogBody = HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(body.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_news_entry_keeps_its_fields_and_resolves_its_image() {
        let raw = r#"
        {
            "entries": [{
                "title": "Minecraft Dungeons II is live",
                "tag": "News",
                "category": "Minecraft Dungeons II",
                "date": "2026-09-28",
                "text": "Time to brave the unknown!",
                "playPageImage": { "title": "dungeons700x466.jpg", "url": "/v2/images/d700.jpg" },
                "newsPageImage": {
                    "title": "dungeons772x350.jpg",
                    "url": "/v2/images/d772.jpg",
                    "dimensions": { "width": 772, "height": 350 }
                },
                "readMoreLink": "https://www.minecraft.net/article/dungeons-ii",
                "newsType": ["Dungeons", "DungeonsII", "News page"],
                "id": "abc"
            }]
        }"#;
        let feed: Feed<RawNewsItem> = serde_json::from_str(raw).expect("the fixture parses");
        let raw = &feed.entries[0];
        assert_eq!(raw.title, "Minecraft Dungeons II is live");
        assert_eq!(raw.news_page_image.url, "/v2/images/d772.jpg");
        assert_eq!(
            absolute_url(&raw.news_page_image.url),
            "https://launchercontent.mojang.com/v2/images/d772.jpg"
        );
        assert_eq!(
            dimensions(&raw.news_page_image, NEWS_IMAGE),
            (772, 350),
            "the feed's own dimensions win"
        );
        assert_eq!(year_month(&raw.date), (2026, 9));
    }

    #[test]
    fn a_document_without_dimensions_falls_back_to_the_source_s_shape() {
        let raw: RawImage = serde_json::from_str(r#"{ "url": "/x.jpg" }"#).expect("parses");
        assert_eq!(dimensions(&raw, CHANGELOG_IMAGE), (540, 540));
    }

    #[test]
    fn an_absolute_url_is_left_alone() {
        // The article link is already a full URL; prefixing it would produce a
        // host with no scheme.
        assert_eq!(
            absolute_url("https://www.minecraft.net/article/x"),
            "https://www.minecraft.net/article/x"
        );
    }

    #[test]
    fn a_changelog_entry_splits_its_timestamp_and_its_kind() {
        let raw = r#"
        {
            "entries": [{
                "title": "Minecraft 26.4 Snapshot 2",
                "version": "26.4-snapshot-2",
                "type": "snapshot",
                "image": { "title": "26.4-snapshot-2 540x540.jpg", "url": "/v2/images/x.jpg" },
                "contentPath": "javaPatchNotes/26-4-snapshot-2.json",
                "id": "26-4-snapshot-2",
                "date": "2026-09-29T11:32:57.000Z",
                "shortText": "Hi there!",
                "needsTranslation": false
            }]
        }"#;
        let feed: Feed<RawChangelogEntry> = serde_json::from_str(raw).expect("the fixture parses");
        let raw = &feed.entries[0];
        assert_eq!(raw.kind, "snapshot");
        assert_eq!(ChangelogKind::from_raw(&raw.kind), ChangelogKind::Snapshot);
        assert_eq!(ChangelogKind::Release.key(), "release");
        assert_eq!(year_month(&raw.date), (2026, 9));
        assert_eq!(raw.content_path, "javaPatchNotes/26-4-snapshot-2.json");
    }

    #[test]
    fn an_unknown_kind_is_a_snapshot() {
        assert_eq!(ChangelogKind::from_raw("preview"), ChangelogKind::Snapshot);
        assert_eq!(ChangelogKind::from_raw("release"), ChangelogKind::Release);
    }

    #[test]
    fn a_malformed_date_yields_no_year_rather_than_a_panic() {
        assert_eq!(year_month(""), (0, 0));
        assert_eq!(year_month("soon"), (0, 0));
    }
}
