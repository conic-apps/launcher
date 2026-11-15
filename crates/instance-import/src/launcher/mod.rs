// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The instance half of the crate: another launcher's instance, read off the
//! folder it keeps on disk instead of a modpack manifest.

mod version_dir;

use std::path::Path;

use crate::error::Result;
use crate::{Metadata, Resolved};

/// A launcher-instance layout this crate reads.
#[derive(Clone)]
pub(crate) enum Instance {
    VersionDir(version_dir::Inspection),
}

impl Instance {
    pub(crate) fn resolve(self) -> Resolved {
        match self {
            Self::VersionDir(inspection) => inspection.resolve(),
        }
    }
}

pub(crate) fn inspect(path: &Path) -> Result<Option<(Metadata, Instance)>> {
    Ok(version_dir::inspect(path)?
        .map(|(metadata, inspection)| (metadata, Instance::VersionDir(inspection))))
}
