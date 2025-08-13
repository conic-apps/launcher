// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type DataLocation = {
    root: string
    instances: string
    cache: string
    default_jre: string
    logs: string
    resources: string
    temp: string
    config: string
}

export async function getDataLocation(): Promise<DataLocation> {
    return await invoke("plugin:folder|cmd_get_data_location")
}
