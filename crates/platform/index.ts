// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type PlatformInfo = {
    arch: string
    os_type: string
    os_family: "Windows" | "Linux" | "Macos"
    os_version: NonNullable<object>
    edition?: string
}

export async function getPlatformInfo(): Promise<PlatformInfo> {
    return await invoke("plugin:platform|cmd_get_platform_info")
}
