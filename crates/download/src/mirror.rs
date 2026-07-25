// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use config::download::MirrorConfig;
use serde::{Deserialize, Serialize};

pub(crate) struct Mirror(pub(crate) String, pub(crate) Arc<AtomicU64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MirrorUsage {
    pub(crate) libraries: HashMap<String, Arc<AtomicU64>>,
    pub(crate) assets: HashMap<String, Arc<AtomicU64>>,
}

// TODO: concurrent download return total bytes and bytes progress

impl MirrorUsage {
    pub(crate) fn new(mirror_config: &MirrorConfig) -> Self {
        Self {
            libraries: mirror_config
                .libraries
                .iter()
                .map(|x| (x.to_string(), Arc::new(AtomicU64::new(0))))
                .collect(),
            assets: mirror_config
                .assets
                .iter()
                .map(|x| (x.to_string(), Arc::new(AtomicU64::new(0))))
                .collect(),
        }
    }
    /// Get a fewest connections libraries mirror
    pub(crate) fn get_libraries_mirror(&self, disabled: &[String]) -> Option<Mirror> {
        let (k, v) = self
            .libraries
            .iter()
            .filter(|x| !disabled.iter().any(|y| x.0 == y))
            .min_by(|x, y| x.1.load(Ordering::SeqCst).cmp(&y.1.load(Ordering::SeqCst)))?;
        Some(Mirror(k.clone(), v.clone()))
    }
    /// Get a fewest connections assets mirror
    pub(crate) fn get_assets_mirror(&self, disabled: &[String]) -> Option<Mirror> {
        let (k, v) = self
            .assets
            .iter()
            .filter(|x| !disabled.iter().any(|y| x.0 == y))
            .min_by(|x, y| x.1.load(Ordering::SeqCst).cmp(&y.1.load(Ordering::SeqCst)))?;
        Some(Mirror(k.clone(), v.clone()))
    }
}
