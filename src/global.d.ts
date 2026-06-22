// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { PlatformInfo } from "@conic/platform"

declare global {
    interface Window {
        __PLATFORM__: PlatformInfo
        __SYSTEM_FONTS__: string[]
    }
}
