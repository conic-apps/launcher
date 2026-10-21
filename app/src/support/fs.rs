// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! File-system helpers shared by `usecases/` and `ui/`.

use std::path::Path;

/// Copies `from` to `to`, creating `to` and any missing parent directories.
///
/// `from` may be a file or a directory; a directory is merged into `to` rather
/// than replacing it, so the caller can copy several trees over one root.
pub(crate) fn copy_recursively(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursively(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
    }
    Ok(())
}
