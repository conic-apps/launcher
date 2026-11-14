// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! UI-neutral data for the content browser: the cards a search or a local scan
//! builds, the search bookkeeping and the open list's query. Turning a card into
//! a Slint `ContentCard` is the adapter's job and stays in
//! `ui/overlays/content`.

use std::collections::{HashMap, HashSet};

use content::remote::{ContentKind, InstalledFile, RemotePlatform};

/// The kind of remote list showing; only one list is ever open.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteKind {
    Mods,
    ResourcePacks,
    Packs,
}

impl RemoteKind {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::Packs => "packs",
        }
    }

    /// CurseForge's `classId`.
    pub(crate) fn curseforge_class(self) -> i64 {
        match self {
            Self::Mods => 6,
            Self::ResourcePacks => 12,
            Self::Packs => 4471,
        }
    }

    /// The Modrinth `project_type` facet.
    pub(crate) fn modrinth_type(self) -> &'static str {
        match self {
            Self::Mods => "mod",
            Self::ResourcePacks => "resourcepack",
            Self::Packs => "modpack",
        }
    }

    /// Whether the kind's cards carry loader tags and use the loader filter.
    pub(crate) fn has_loaders(self) -> bool {
        matches!(self, Self::Mods | Self::Packs)
    }

    /// Where a download of this kind lands, under the instance root.
    pub(crate) fn folder(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::Packs => "modpacks",
        }
    }

    /// The content kind this remote list installs. A modpack is not instance
    /// content, so it has no shared installed-content scope.
    pub(crate) fn content_kind(self) -> Option<ContentKind> {
        match self {
            Self::Mods => Some(ContentKind::Mods),
            Self::ResourcePacks => Some(ContentKind::ResourcePacks),
            Self::Packs => None,
        }
    }

    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "resourcepacks" => Self::ResourcePacks,
            "packs" => Self::Packs,
            _ => Self::Mods,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Modrinth,
    CurseForge,
}

impl Platform {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curseforge",
        }
    }

    pub(crate) fn api(self) -> RemotePlatform {
        match self {
            Self::Modrinth => RemotePlatform::Modrinth,
            Self::CurseForge => RemotePlatform::CurseForge,
        }
    }
}

/// A card as the background work builds it: plain data, and an icon as a
/// [`PendingImage`], so it can cross into an `upgrade_in_event_loop` closure that
/// a Slint `ContentCard` cannot.
#[derive(Clone, Default)]
pub(crate) struct PendingCard {
    pub(crate) id: String,
    pub(crate) link: String,
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) has_subtitle: bool,
    pub(crate) description: String,
    pub(crate) tags: Vec<PendingTag>,
    /// The icon, when it is already decoded — a `data:` URL, which costs nothing
    /// to decode, or an image this same background pass produced.
    pub(crate) icon: Option<PendingImage>,
    /// Where the icon comes from, when it has not been decoded yet: a local
    /// `data:` URL or a remote `https:` one. Also the card's loading state: with
    /// no icon at all the card shows the unknown-server texture, while this one
    /// shows a spinner until the fetch lands or fails.
    pub(crate) icon_url: Option<String>,
    /// "" | "favorite" | "file"
    pub(crate) action_kind: &'static str,
    pub(crate) shows_play: bool,
    pub(crate) mod_disabled: bool,
    /// A save's spawn point, which its world map opens centred on. Zero on
    /// every other kind of card, and on a save whose `level.dat` has no
    /// `spawn.pos` — which asks the world for its own instead.
    pub(crate) spawn: Option<(i32, i32)>,
}

/// An image on its way to the UI thread: a plain buffer, because Slint's `Image`
/// is not `Send`. The decode happens on the background thread; building the
/// `Image` from the buffer on the UI thread is a copy.
#[derive(Clone)]
pub(crate) struct PendingImage {
    /// The URL or path it came from, which is also its cache key.
    pub(crate) key: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

/// One tag, before it becomes a Slint struct.
#[derive(Clone)]
pub(crate) struct PendingTag {
    pub(crate) text: String,
    pub(crate) label: String,
    pub(crate) kind: String,
    /// A relative time's `(kind, hours, month, day, year)`, for the saves'
    /// last-played tag — which is translated on the Slint side.
    pub(crate) time: Option<(&'static str, i32, i32, i32, i32)>,
}

/// What a card's id stands for, so a callback carrying only the id can find the
/// thing behind it. Rebuilt whenever a list is loaded.
#[derive(Clone)]
pub(crate) enum CardTarget {
    Remote {
        platform: Platform,
        id: String,
    },
    /// A local file — a mod jar or a resource pack — whose folder the card
    /// opens.
    Path(String),
    /// A save folder.
    Save(String),
}

/// The open remote list's query and selections, the same shape for all six
/// lists.
#[derive(Clone, Default)]
pub(crate) struct SearchForm {
    pub(crate) query: String,
    pub(crate) loaders: Vec<String>,
    pub(crate) versions: Vec<String>,
    /// Modrinth category slugs, or CurseForge category ids as strings.
    pub(crate) categories: Vec<String>,
    pub(crate) favorites_only: bool,
    pub(crate) page: usize,
}

/// A card and the targets its callbacks need, which only the search knows.
#[derive(Clone)]
pub(crate) struct BuiltCard {
    pub(crate) card: PendingCard,
    pub(crate) target: CardTarget,
}

/// One remote list's search bookkeeping. A source switch destroys and re-creates
/// the list, which keeps this.
#[derive(Default)]
pub(crate) struct ListSearch {
    /// The request key → what that exact request returned: the cards and the
    /// total hit count the page count comes from. A page that has been seen is
    /// drawn without the spinner.
    pub(crate) cache: HashMap<String, (Vec<BuiltCard>, usize)>,
    /// The newest request's number. An answer that belongs to an older request
    /// is dropped rather than drawn over a newer one.
    pub(crate) token: u64,
    /// The instance runtime this list last seeded its filters from, as
    /// `"<loader>|<minecraft>"`. A list seeds once per instance.
    pub(crate) initialized_for: Option<String>,
}

/// The detail panel that is open, and what its buttons act on.
#[derive(Clone)]
pub(crate) struct OpenDetail {
    pub(crate) platform: Platform,
    pub(crate) kind: RemoteKind,
    pub(crate) id: String,
    /// The installed files the project resolved to, for the remove button and
    /// the installed version label.
    pub(crate) installed_files: Vec<InstalledFile>,
}

/// `<platform>:<kind>:<id>` — the favourites key.
pub(crate) fn favorite_key(platform: &str, kind: &str, id: &str) -> String {
    format!("{platform}:{kind}:{id}")
}

/// The content browser's session state: what list is open, its query, the search
/// cache and page numbers, the favourites and the open detail. Plain data, so a
/// second frontend can own the same state; the Slint models and the measured
/// layout live in the adapter.
pub(crate) struct ContentSession {
    /// The game view's current instance. Re-read whenever a panel opens.
    pub(crate) instance_id: String,
    /// `<platform>:<kind>:<id>` — the favourites key.
    pub(crate) favorites: HashSet<String>,
    pub(crate) targets: HashMap<String, CardTarget>,
    pub(crate) kind: RemoteKind,
    pub(crate) platform: Platform,
    /// "" | "local" | "modrinth" | "curseforge"
    pub(crate) source: String,
    /// Bumped whenever the source changes, so an instance-runtime lookup cannot
    /// apply after a newer switch has replaced it.
    pub(crate) source_generation: u64,
    pub(crate) form: SearchForm,
    /// The Minecraft release list, newest first. Empty until it arrives.
    pub(crate) version_options: Vec<String>,
    /// `<kind>:<source>` → that list's `(current page, total pages)`; every list
    /// has its own pair.
    pub(crate) pages: HashMap<String, (usize, usize)>,
    /// `<kind>:<source>` → that list's own search cache and request token.
    pub(crate) lists: HashMap<String, ListSearch>,
    /// Translated project descriptions, keyed `<platform>:<id>`; only for a
    /// Chinese locale, and only for the ids that have been on screen.
    pub(crate) translations: HashMap<String, String>,
    pub(crate) detail: Option<OpenDetail>,
    /// Bumped on every open, so a slow detail response cannot overwrite a newer
    /// panel.
    pub(crate) detail_seq: u64,
}

impl Default for ContentSession {
    fn default() -> Self {
        Self {
            instance_id: String::new(),
            favorites: HashSet::new(),
            targets: HashMap::new(),
            kind: RemoteKind::Mods,
            platform: Platform::Modrinth,
            source: "local".into(),
            source_generation: 0,
            form: SearchForm::default(),
            version_options: Vec::new(),
            pages: HashMap::new(),
            lists: HashMap::new(),
            translations: HashMap::new(),
            detail: None,
            detail_seq: 0,
        }
    }
}

impl ContentSession {
    pub(crate) fn is_favorited(&self, platform: Platform, id: &str) -> bool {
        self.favorites
            .contains(&favorite_key(platform.key(), self.kind.key(), id))
    }
}
