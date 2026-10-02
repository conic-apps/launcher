// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The directory the per-instance game folders live in.

use std::path::{Path, PathBuf};

/// The instances root. Every instance is one directory under it, holding its
/// `instance.toml` beside the game's own `mods`, `saves` and so on.
#[derive(Debug, Clone)]
pub struct InstancesLocation {
    pub root: PathBuf,
}

impl InstancesLocation {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn get_instance_root(&self, instance_id: &str) -> PathBuf {
        self.root.join(instance_id)
    }

    pub fn init(&self) {
        let _ = std::fs::create_dir_all(&self.root);
    }
}
