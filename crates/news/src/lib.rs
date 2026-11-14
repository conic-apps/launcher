// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launcher-content client: Mojang's news feed and the Java Edition
//! changelogs, both unauthenticated static JSON from `launchercontent.mojang.com`.
//! Sorting newest-first lives here because it is a property of the feed rather
//! than of a view.

pub mod error;

use error::*;
use serde::Deserialize;
use shared::HTTP_CLIENT;

const BASE_URL: &str = "https://launchercontent.mojang.com";

/// `image_url` and `read_more` are already absolute.
#[derive(Debug, Clone)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    /// The edition as the feed spells it, shown as the card's category.
    pub category: String,
    /// The platform tags, kept so a caller can narrow the feed without
    /// re-parsing it.
    pub news_type: Vec<String>,
    /// The feed's `YYYY-MM-DD`.
    pub date: String,
    /// `date` split out: the year and month drive the filters, and all three let
    /// the app format the date for the reader's locale.
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub text: String,
    pub image_url: String,
    /// The banner's own dimensions; a document without them falls back to
    /// 772×350.
    pub image_width: u32,
    pub image_height: u32,
    pub read_more: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogKind {
    Release,
    Snapshot,
}

impl ChangelogKind {
    /// The machine-readable key, so a view can resolve a translated label
    /// without knowing the Rust enum.
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

#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub kind: ChangelogKind,
    pub date: String,
    /// `date` split out, for the same reason as [`NewsItem`].
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub image_url: String,
    /// The changelog index carries no dimensions, so these are 540×540 unless a
    /// document grows them.
    pub image_width: u32,
    pub image_height: u32,
    pub short_text: String,
    /// The full body's location relative to `/v2/`, e.g.
    /// `javaPatchNotes/26-4-snapshot-2.json`.
    pub content_path: String,
}

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

/// The changelog index prints no dimensions, so the field is optional.
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
/// Every changelog image is square; the card only reads the ratio.
const CHANGELOG_IMAGE: (u32, u32) = (540, 540);

#[derive(Debug, Deserialize)]
struct Feed<T> {
    entries: Vec<T>,
}

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

/// `YYYY-MM-DD` or an ISO timestamp into `(year, month, day)`.
///
/// A split rather than a date parser: both feeds put the date first and
/// zero-padded, so this cannot fail or drift with a locale. An unparseable
/// component becomes 0, which makes the entry disappear from every date filter,
/// hence the `debug!`.
fn date_parts(date: &str) -> (u16, u8, u8) {
    let mut parts = date.split(['-', 'T']);
    let year = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let month = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let day = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    if year == 0 || month == 0 {
        log::debug!("an entry has an unreadable date {date:?}; it will not match a date filter");
    }
    (year, month, day)
}

/// Fetches `url` and decodes it as JSON, recording what the server said. These
/// requests bypass `shared::http_cache`, so the status code is what tells a proxy
/// error page apart from Mojang being down.
async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    let response = HTTP_CLIENT.get(url).send().await.map_err(|error| {
        log::warn!("{url} could not be reached: {error}");
        Error::Network(error)
    })?;
    let status = response.status();
    if !status.is_success() {
        match response.headers().get(reqwest::header::RETRY_AFTER) {
            Some(retry_after) => log::warn!(
                "{url} answered {status} and asked to retry after {}",
                retry_after
                    .to_str()
                    .unwrap_or("<an unreadable Retry-After>")
            ),
            None => log::warn!("{url} answered {status}"),
        }
    }
    response.json().await.map_err(|error| {
        log::warn!("{url} answered {status} but the body did not parse as JSON: {error}");
        Error::Network(error)
    })
}

pub async fn fetch_news() -> Result<Vec<NewsItem>> {
    let url = format!("{BASE_URL}/v2/news.json");
    let feed: Feed<RawNewsItem> = fetch_json(&url).await?;
    let mut items: Vec<NewsItem> = feed
        .entries
        .into_iter()
        .map(|raw| {
            let (year, month, day) = date_parts(&raw.date);
            let (image_width, image_height) = dimensions(&raw.news_page_image, NEWS_IMAGE);
            NewsItem {
                id: raw.id,
                title: raw.title,
                category: raw.category,
                news_type: raw.news_type,
                date: raw.date,
                year,
                month,
                day,
                text: raw.text,
                image_url: absolute_url(&raw.news_page_image.url),
                image_width,
                image_height,
                read_more: absolute_url(&raw.read_more_link),
            }
        })
        .collect();
    // The feeds are close to sorted, but relying on that would put a re-ordered
    // document on screen in the wrong order.
    items.sort_by(|a, b| b.date.cmp(&a.date));
    // An empty browser and a failed fetch look the same on screen, so the count
    // is the only useful success signal a feed has.
    log::info!("Mojang's news feed returned {} entries", items.len());
    Ok(items)
}

pub async fn fetch_changelogs() -> Result<Vec<ChangelogEntry>> {
    let url = format!("{BASE_URL}/v2/javaPatchNotes.json");
    let feed: Feed<RawChangelogEntry> = fetch_json(&url).await?;
    let mut items: Vec<ChangelogEntry> = feed
        .entries
        .into_iter()
        .map(|raw| {
            let (year, month, day) = date_parts(&raw.date);
            let (image_width, image_height) = dimensions(&raw.image, CHANGELOG_IMAGE);
            ChangelogEntry {
                id: raw.id,
                title: raw.title,
                version: raw.version,
                kind: ChangelogKind::from_raw(&raw.kind),
                date: raw.date,
                year,
                month,
                day,
                image_url: absolute_url(&raw.image.url),
                image_width,
                image_height,
                short_text: raw.short_text,
                content_path: raw.content_path,
            }
        })
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    log::info!("Mojang's changelog index returned {} entries", items.len());
    Ok(items)
}

/// The full HTML of one changelog, read from the `contentPath` its index entry
/// carries.
pub async fn fetch_changelog_body(content_path: &str) -> Result<String> {
    let url = format!("{BASE_URL}/v2/{content_path}");
    // `content_path` comes straight out of the index document and is interpolated
    // into the URL, so the resolved URL is recorded: it is the only way to tell
    // which changelog was asked for, and what a surprising one resolved to.
    let body: RawChangelogBody = fetch_json(&url).await?;
    log::debug!(
        "read the changelog body for {content_path} ({} bytes)",
        body.body.len()
    );
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
        assert_eq!(date_parts(&raw.date), (2026, 9, 28));
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
        assert_eq!(date_parts(&raw.date), (2026, 9, 29));
        assert_eq!(raw.content_path, "javaPatchNotes/26-4-snapshot-2.json");
    }

    #[test]
    fn an_unknown_kind_is_a_snapshot() {
        assert_eq!(ChangelogKind::from_raw("preview"), ChangelogKind::Snapshot);
        assert_eq!(ChangelogKind::from_raw("release"), ChangelogKind::Release);
    }

    #[test]
    fn a_malformed_date_yields_no_year_rather_than_a_panic() {
        assert_eq!(date_parts(""), (0, 0, 0));
        assert_eq!(date_parts("soon"), (0, 0, 0));
    }
}
