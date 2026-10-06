// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The news browser's session.
//!
//! UI-neutral, like every other use case: it holds the two feeds the
//! `news` crate fetched and the filter over them (the feed kind, a year, a
//! month and a search string), and it answers "which entries match" and "which
//! years can be picked". Nothing here mentions Slint, the `ui/` modules or how a
//! card is drawn; the `ui/overlays/news` adapter owns the models and the
//! geometry and drives this.

use news::{ChangelogEntry, NewsItem};

/// Which of the two feeds the panel is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum NewsKind {
    /// The Mojang news feed — every edition, as the feed carries it.
    #[default]
    News,
    /// The Java Edition changelog index.
    Changelog,
}

impl NewsKind {
    /// The key the filter chip and the card model share.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::News => "news",
            Self::Changelog => "changelog",
        }
    }

    /// The inverse of [`Self::key`]; anything but "changelog" is the news feed,
    /// which is the default the panel opens on.
    pub(crate) fn from_key(key: &str) -> Self {
        if key == "changelog" {
            Self::Changelog
        } else {
            Self::News
        }
    }
}

/// The feeds and the filter over them.
#[derive(Default)]
pub(crate) struct NewsSession {
    news: Vec<NewsItem>,
    changelogs: Vec<ChangelogEntry>,
    loaded: bool,
    loading: bool,
    kind: NewsKind,
    year: Option<u16>,
    month: Option<u8>,
    query: String,
}

impl NewsSession {
    pub(crate) fn kind(&self) -> NewsKind {
        self.kind
    }

    /// Switching feeds clears the year, because the two feeds do not share
    /// their year lists (the news feed reaches back to 2025, the changelog to
    /// 2018) — a year left over from the other list would filter everything
    /// out the moment the switch happened.
    pub(crate) fn set_kind(&mut self, kind: NewsKind) {
        if self.kind == kind {
            return;
        }
        self.kind = kind;
        self.year = None;
    }

    pub(crate) fn year(&self) -> Option<u16> {
        self.year
    }

    pub(crate) fn set_year(&mut self, year: Option<u16>) {
        self.year = year;
    }

    pub(crate) fn month(&self) -> Option<u8> {
        self.month
    }

    pub(crate) fn set_month(&mut self, month: Option<u8>) {
        self.month = month;
    }

    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub(crate) fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Claims the one fetch. Returns false when one is already in flight or has
    /// already landed, so a second open is a no-op.
    pub(crate) fn begin_load(&mut self) -> bool {
        if self.loaded || self.loading {
            return false;
        }
        self.loading = true;
        true
    }

    /// Accepts the two feeds. Either may be absent if its request failed; the
    /// other still shows, and a failed one is not retried on the next open.
    pub(crate) fn finish_load(
        &mut self,
        news: Result<Vec<NewsItem>, String>,
        changelogs: Result<Vec<ChangelogEntry>, String>,
    ) {
        match news {
            Ok(items) => self.news = items,
            Err(error) => log::error!("failed to fetch the Mojang news feed: {error}"),
        }
        match changelogs {
            Ok(items) => self.changelogs = items,
            Err(error) => log::error!("failed to fetch the Java changelogs: {error}"),
        }
        self.loading = false;
        self.loaded = true;
    }

    /// The entries of the active feed that pass the year, month and query, in
    /// the feeds' own newest-first order.
    pub(crate) fn filtered_news(&self) -> impl Iterator<Item = &NewsItem> {
        self.news
            .iter()
            .filter(|item| self.passes(item.year, item.month, &item.title, &item.text))
    }

    /// See [`Self::filtered_news`].
    pub(crate) fn filtered_changelogs(&self) -> impl Iterator<Item = &ChangelogEntry> {
        self.changelogs
            .iter()
            .filter(|entry| self.passes(entry.year, entry.month, &entry.title, &entry.short_text))
    }

    /// The years the active feed actually has, newest first. The chips are built
    /// from this rather than from a fixed range, so a year never yields an
    /// empty list.
    pub(crate) fn years(&self) -> Vec<u16> {
        let years: Vec<u16> = match self.kind {
            NewsKind::News => self.news.iter().map(|item| item.year).collect(),
            NewsKind::Changelog => self.changelogs.iter().map(|entry| entry.year).collect(),
        };
        let mut years: Vec<u16> = years.into_iter().filter(|year| *year > 0).collect();
        years.sort_unstable();
        years.dedup();
        years.reverse();
        years
    }

    pub(crate) fn news_item(&self, id: &str) -> Option<&NewsItem> {
        self.news.iter().find(|item| item.id == id)
    }

    pub(crate) fn changelog(&self, id: &str) -> Option<&ChangelogEntry> {
        self.changelogs.iter().find(|entry| entry.id == id)
    }

    /// Whether a dated entry passes the three filters. Kept apart from the two
    /// iterators so the news and changelog rules cannot drift apart.
    fn passes(&self, year: u16, month: u8, title: &str, body: &str) -> bool {
        if let Some(wanted) = self.year
            && year != wanted
        {
            return false;
        }
        if let Some(wanted) = self.month
            && month != wanted
        {
            return false;
        }
        let query = self.query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        title.to_lowercase().contains(&query) || body.to_lowercase().contains(&query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn news_item(id: &str, year: u16, month: u8, title: &str, text: &str) -> NewsItem {
        NewsItem {
            id: id.into(),
            title: title.into(),
            category: "Minecraft: Java Edition".into(),
            news_type: vec!["Java".into()],
            date: format!("{year:04}-{month:02}-01"),
            year,
            month,
            text: text.into(),
            image_url: String::new(),
            image_width: 772,
            image_height: 350,
            read_more: String::new(),
        }
    }

    fn changelog(id: &str, year: u16, month: u8, title: &str) -> ChangelogEntry {
        ChangelogEntry {
            id: id.into(),
            title: title.into(),
            version: id.into(),
            kind: ::news::ChangelogKind::Snapshot,
            date: format!("{year:04}-{month:02}-01T00:00:00.000Z"),
            year,
            month,
            image_url: String::new(),
            image_width: 540,
            image_height: 540,
            short_text: String::new(),
            content_path: format!("javaPatchNotes/{id}.json"),
        }
    }

    fn loaded() -> NewsSession {
        let mut session = NewsSession::default();
        session.finish_load(
            Ok(vec![
                news_item("a", 2026, 9, "Dungeons II is live", "brave the unknown"),
                news_item("b", 2026, 3, "Spring update", "flowers"),
                news_item("c", 2025, 12, "Winter event", "snow"),
            ]),
            Ok(vec![
                changelog("d", 2026, 9, "Snapshot 2"),
                changelog("e", 2018, 7, "1.13"),
            ]),
        );
        session
    }

    #[test]
    fn a_feed_is_fetched_once() {
        let mut session = NewsSession::default();
        assert!(session.begin_load());
        assert!(!session.begin_load(), "a second open must not refetch");
        session.finish_load(Ok(vec![]), Ok(vec![]));
        assert!(!session.begin_load(), "a loaded session must not refetch");
    }

    #[test]
    fn the_year_filter_narrows_to_the_active_feed() {
        let mut session = loaded();
        session.set_kind(NewsKind::News);
        session.set_year(Some(2026));
        let ids: Vec<&str> = session
            .filtered_news()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn switching_feed_clears_the_year_but_keeps_the_query() {
        let mut session = loaded();
        session.set_kind(NewsKind::News);
        session.set_year(Some(2026));
        session.set_query("snapshot".into());
        session.set_kind(NewsKind::Changelog);
        assert_eq!(session.year(), None, "the year belonged to the other feed");
        let ids: Vec<&str> = session
            .filtered_changelogs()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(ids, ["d"]);
    }

    #[test]
    fn the_month_filter_is_exact() {
        let mut session = loaded();
        session.set_kind(NewsKind::News);
        session.set_month(Some(3));
        let ids: Vec<&str> = session
            .filtered_news()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(ids, ["b"]);
    }

    #[test]
    fn the_query_matches_the_title_or_the_body_without_case() {
        let mut session = loaded();
        session.set_kind(NewsKind::News);
        session.set_query("FLOWERS".into());
        let ids: Vec<&str> = session
            .filtered_news()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(ids, ["b"], "the body matched");
    }

    #[test]
    fn the_year_chips_come_from_the_active_feed_newest_first() {
        let mut session = loaded();
        session.set_kind(NewsKind::News);
        assert_eq!(session.years(), [2026, 2025]);
        session.set_kind(NewsKind::Changelog);
        assert_eq!(session.years(), [2026, 2018]);
    }

    #[test]
    fn a_feeds_entries_are_found_by_id() {
        let session = loaded();
        assert!(session.news_item("a").is_some());
        assert!(session.changelog("d").is_some());
        assert!(
            session.news_item("d").is_none(),
            "the feeds do not share ids"
        );
    }
}
