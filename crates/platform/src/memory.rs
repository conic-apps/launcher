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
    let available = system.available_memory();
    // Zero means `sysinfo` could not read the host at all, and the automatic
    // heap calculation then computes an `-Xmx` of zero — the game starts and
    // immediately fails, with nothing in the log to connect the two.
    if available == 0 {
        log::warn!("the available system memory could not be read (reported as 0 bytes)");
    }
    available
}
