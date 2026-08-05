// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { DataLocation } from "@conic/folder"
import { PlatformInfo } from "@conic/platform"

declare global {
    interface Window {
        __PLATFORM__: PlatformInfo
        __DATA_LOCATION__: DataLocation
    }
}
