// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! UI-neutral data for the content browser.
//!
//! The cards a search or a local scan builds, the search bookkeeping (its cache
//! and request token) and the open list's query are all plain data. They live
//! here, not in the Slint adapter, so the same cache and paging logic can back a
//! second frontend. Turning a [`PendingCard`] into the Slint `ContentCard` is
//! the adapter's job and stays in `ui/overlays/content`.

use std::collections::{HashMap, HashSet};

use content::mods::ResolvedMod;
use content::mods::remote::RemoteModPlatform;

use crate::usecases::generation::Gate;

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

    pub(crate) fn api(self) -> RemoteModPlatform {
        match self {
            Self::Modrinth => RemoteModPlatform::Modrinth,
            Self::CurseForge => RemoteModPlatform::CurseForge,
        }
    }
}

/// A card as the background work builds it.
///
/// A Slint model is not `Send`, so a `ContentCard` — whose `tags` are a
/// `ModelRc` — cannot be moved into an `upgrade_in_event_loop` closure. The
/// built cards therefore travel as plain data and become `ContentCard`s inside
/// the closure, on the UI thread. An icon travels as a [`PendingImage`] — a
/// plain RGBA buffer — because Slint's `Image` itself is not `Send` either.
///
/// [`PendingCard::icon`] is therefore almost always `None` and
/// [`PendingCard::icon_url`] carries where the icon will come from instead. The
/// cards go to the model first and their icons are filled in as the bytes arrive
/// (see `content::icon::load_icons`): a page of remote results is all `https:`
/// URLs, and fetching them before the first card is drawn is what leaves a list
/// looking dead for as long as the slowest of them takes.
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
    /// `data:` URL or a remote `https:` one. It doubles as the card's loading
    /// state — a card with no icon at all shows the unknown-server texture right
    /// away, while this one shows a spinner until the fetch lands or fails.
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

/// An image on its way to the UI thread.
///
/// Slint's `Image` is not `Send`, so a decoded image cannot cross into an
/// `upgrade_in_event_loop` closure. The *decode* — the expensive half, and the
/// one that would stall the window — happens on the background thread, and what
/// crosses is its result: a plain buffer. Building the `Image` from it on the
/// UI thread is a copy.
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

/// One remote list's search bookkeeping.
///
/// There is one of each per list, and a list that is destroyed and re-created
/// by a source switch keeps the cache it built.
#[derive(Default)]
pub(crate) struct ListSearch {
    /// The request key → what that exact request returned: the cards and the
    /// total hit count the page count comes from. A page that has been seen is
    /// drawn straight away, without the spinner a fresh request shows.
    pub(crate) cache: HashMap<String, (Vec<BuiltCard>, usize)>,
    /// The newest request's number. An answer that belongs to an older request
    /// is dropped rather than drawn over a newer one — which is what rapid
    /// paging needs, since the answers do not come back in order.
    pub(crate) token: u64,
    /// The instance runtime this list last seeded its filters from, as
    /// `"<loader>|<minecraft>"`. A list seeds once per instance and not again,
    /// so switching away and back gives a list with no filters on it while the
    /// cache above survives.
    pub(crate) initialized_for: Option<String>,
}

/// The detail panel that is open, and what its buttons act on.
#[derive(Clone)]
pub(crate) struct OpenDetail {
    pub(crate) platform: Platform,
    pub(crate) kind: RemoteKind,
    pub(crate) id: String,
    /// The installed mods the project resolved to, for the remove button.
    pub(crate) installed_mods: Vec<ResolvedMod>,
}

/// `<platform>:<kind>:<id>` — the favourites key.
pub(crate) fn favorite_key(platform: &str, kind: &str, id: &str) -> String {
    format!("{platform}:{kind}:{id}")
}

/// The content browser's session state: what list is open, its query, the
/// search cache and page numbers, the favourites and the open detail.
///
/// Plain data, so a second frontend can own the same state and run the same
/// logic. The Slint models and the measured layout live in the adapter.
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
    pub(crate) form: SearchForm,
    /// The Minecraft release list, newest first. Empty until it arrives.
    pub(crate) version_options: Vec<String>,
    /// `<kind>:<source>` → that list's `(current page, total pages)`.
    ///
    /// Every list has its own pair, and a source switch destroys and re-creates
    /// one of them, so one list's page numbers are never shown against another's
    /// results.
    pub(crate) pages: HashMap<String, (usize, usize)>,
    /// `<kind>:<source>` → that list's own search cache and request token.
    pub(crate) lists: HashMap<String, ListSearch>,
    /// Translated project descriptions, keyed `<platform>:<id>`. Only filled
    /// for a Chinese locale, and only for the ids that have been on screen.
    pub(crate) translations: HashMap<String, String>,
    pub(crate) detail: Option<OpenDetail>,
    /// Bumped on every open, so a slow detail response cannot overwrite a newer
    /// panel.
    pub(crate) detail_seq: u64,
    /// Issues the detail download's token and invalidates it on a newer one.
    pub(crate) detail_download_gate: Gate,
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
            form: SearchForm::default(),
            version_options: Vec::new(),
            pages: HashMap::new(),
            lists: HashMap::new(),
            translations: HashMap::new(),
            detail: None,
            detail_seq: 0,
            detail_download_gate: Gate::new(),
        }
    }
}

impl ContentSession {
    pub(crate) fn is_favorited(&self, platform: Platform, id: &str) -> bool {
        self.favorites
            .contains(&favorite_key(platform.key(), self.kind.key(), id))
    }
}
