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

use std::collections::HashMap;

use content::mods::ResolvedMod;
use content::mods::remote::RemoteModPlatform;

/// Which remote list is showing: the kind and the platform are state, because
/// only one is ever open.
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

    /// CurseForge's `classId`: 6 mods, 12 resource packs, 4471 modpacks.
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
    /// A resource pack has no loader.
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
#[derive(Clone, Default)]
pub(crate) struct PendingCard {
    pub(crate) id: String,
    pub(crate) link: String,
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) has_subtitle: bool,
    pub(crate) description: String,
    pub(crate) tags: Vec<PendingTag>,
    pub(crate) icon: Option<PendingImage>,
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
