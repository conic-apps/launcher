// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use config::download::MirrorConfig;
use log::debug;
use serde::{Deserialize, Serialize};

pub(crate) struct Mirror(pub(crate) String, pub(crate) Arc<AtomicU64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MirrorUsage {
    pub(crate) libraries: HashMap<String, Arc<AtomicU64>>,
    pub(crate) assets: HashMap<String, Arc<AtomicU64>>,
}

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
    /// Selects the libraries mirror with the fewest active connections.
    pub(crate) fn get_libraries_mirror(&self, disabled: &[String]) -> Option<Mirror> {
        let (k, v) = self
            .libraries
            .iter()
            .filter(|x| !disabled.iter().any(|y| x.0 == y))
            .min_by(|x, y| x.1.load(Ordering::SeqCst).cmp(&y.1.load(Ordering::SeqCst)))?;
        debug!(
            "Selected libraries mirror {k} ({} active connection(s), {} disabled)",
            v.load(Ordering::SeqCst),
            disabled.len()
        );
        Some(Mirror(k.clone(), v.clone()))
    }
    /// Selects the assets mirror with the fewest active connections.
    pub(crate) fn get_assets_mirror(&self, disabled: &[String]) -> Option<Mirror> {
        let (k, v) = self
            .assets
            .iter()
            .filter(|x| !disabled.iter().any(|y| x.0 == y))
            .min_by(|x, y| x.1.load(Ordering::SeqCst).cmp(&y.1.load(Ordering::SeqCst)))?;
        debug!(
            "Selected assets mirror {k} ({} active connection(s), {} disabled)",
            v.load(Ordering::SeqCst),
            disabled.len()
        );
        Some(Mirror(k.clone(), v.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(libraries: &[&str], assets: &[&str]) -> MirrorUsage {
        MirrorUsage::new(&MirrorConfig {
            libraries: libraries.iter().map(|x| x.to_string()).collect(),
            assets: assets.iter().map(|x| x.to_string()).collect(),
        })
    }

    #[test]
    fn no_mirror_configured_selects_nothing() {
        let usage = usage(&[], &[]);
        assert!(usage.get_libraries_mirror(&[]).is_none());
        assert!(usage.get_assets_mirror(&[]).is_none());
    }

    #[test]
    fn the_only_mirror_is_selected() {
        let usage = usage(&["a"], &["b"]);
        assert_eq!(usage.get_libraries_mirror(&[]).unwrap().0, "a");
        assert_eq!(usage.get_assets_mirror(&[]).unwrap().0, "b");
    }

    #[test]
    fn the_mirror_with_the_fewest_connections_is_selected() {
        let usage = usage(&["busy", "idle"], &[]);
        usage.libraries["busy"].store(3, Ordering::SeqCst);
        let selected = usage.get_libraries_mirror(&[]).expect("one is free");
        assert_eq!(selected.0, "idle");
    }

    #[test]
    fn a_disabled_mirror_is_skipped() {
        let usage = usage(&["a", "b"], &[]);
        assert_eq!(
            usage.get_libraries_mirror(&["b".to_string()]).unwrap().0,
            "a"
        );
    }

    #[test]
    fn disabling_every_mirror_selects_nothing() {
        let usage = usage(&["a", "b"], &[]);
        let disabled = ["a".to_string(), "b".to_string()];
        assert!(usage.get_libraries_mirror(&disabled).is_none());
    }
}
