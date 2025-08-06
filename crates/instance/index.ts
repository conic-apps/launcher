// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type InstanceRuntime = {
    minecraft: String
    mod_loader_type?: "Quilt" | "Fabric" | "Neoforged" | "Forge"
    mod_loader_version?: String
}
export type InstanceConfig = {
    name: string
    runtime: InstanceRuntime
    group?: string[]
    launch_config: {
        enable_instance_specific_settings: boolean
        min_memory?: number
        max_memory?: number
        server?: {
            ip: string
            port?: number
        }
        width?: number
        height?: number
        fullscreen?: boolean
        extra_jvm_args?: string
        extra_mc_args?: string
        is_demo?: boolean
        ignore_invalid_minecraft_certificates?: boolean
        ignore_patch_discrepancies?: boolean
        extra_class_paths?: string
        gc?: "Serial" | "Parallel" | "ParallelOld" | "G1" | "Z"
        launcher_name?: string
        wrap_command?: string
        execute_before_launch?: string
        execute_after_launch?: string
    }
}

export type Instance = {
    config: InstanceConfig
    installed: boolean
    id: string
}

export async function createInstance(instanceConfig: InstanceConfig): Promise<Instance> {
    return await invoke("plugin:instance|cmd_create_instance", { config: instanceConfig })
}

export async function listInstances(sortBy: "Name"): Promise<Instance[]> {
    return await invoke("plugin:instance|cmd_list_instance", { sort_by: sortBy })
}

export async function updateInstance(config: InstanceConfig, id: string): Promise<Instance[]> {
    return await invoke("plugin:instance|cmd_update_instance", { config, id })
}

export async function deleteInstance(id: string): Promise<Instance[]> {
    return await invoke("plugin:instance|cmd_delete_instance", { id })
}
