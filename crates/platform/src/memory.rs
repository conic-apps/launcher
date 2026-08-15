// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use sysinfo::System;

/// Returns the currently available physical memory in bytes.
///
/// This is the memory that can be immediately handed over to the game without
/// swapping, similar to the `ullAvailPhys` field of `GlobalMemoryStatusEx`.
pub fn get_available_memory_bytes() -> u64 {
    let mut system = System::new();
    system.refresh_memory();
    system.available_memory()
}
