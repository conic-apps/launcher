// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The content kinds the instance stores and the platforms their files can be
//! resolved through.

use std::path::{Path, PathBuf};

use storage::LOCATIONS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemotePlatform {
    Modrinth,
    CurseForge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentKind {
    Mods,
    ResourcePacks,
    DataPacks,
}

impl ContentKind {
    /// The directory under the instance root, and under the launcher cache.
    pub fn folder(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::DataPacks => "datapacks",
        }
    }

    /// Whether the path takes part in the installed-content index. Resource and
    /// data packs are only queried as zips; a directory pack is still listed
    /// locally, but the installed check deliberately skips it.
    pub fn indexes_file(self, path: &Path) -> bool {
        match self {
            Self::Mods => true,
            Self::ResourcePacks | Self::DataPacks => path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContentScope {
    pub instance_id: String,
    pub kind: ContentKind,
    /// The world a data pack belongs to. Every other kind has no world.
    pub world: Option<String>,
}

impl ContentScope {
    pub fn new(instance_id: impl Into<String>, kind: ContentKind) -> Self {
        Self {
            instance_id: instance_id.into(),
            kind,
            world: None,
        }
    }

    pub fn with_world(instance_id: impl Into<String>, world: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            kind: ContentKind::DataPacks,
            world: Some(world.into()),
        }
    }

    pub fn folder(&self) -> PathBuf {
        let root = LOCATIONS.instances.get_instance_root(&self.instance_id);
        match self.kind {
            ContentKind::Mods => root.join("mods"),
            ContentKind::ResourcePacks => root.join("resourcepacks"),
            ContentKind::DataPacks => root
                .join("saves")
                .join(self.world.as_deref().unwrap_or_default())
                .join("datapacks"),
        }
    }
}
