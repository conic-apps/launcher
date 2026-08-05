// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type DataLocation = {
    root: string
    accounts: string
    authlib_injector: string
    instances: string
    cache: string
    logs: string
    resources: string
    music: string
    runtime: string
    temp: string
    config: string
}

export async function getDataLocation(): Promise<DataLocation> {
    return await invoke("plugin:folder|cmd_get_data_location")
}

export async function getInstanceRoot(instanceId: string): Promise<string> {
    return await invoke("plugin:folder|cmd_get_instance_root", { instanceId })
}
