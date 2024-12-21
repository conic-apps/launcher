// Amethyst Launcher
// Copyright 2022-2025 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"

export type Instance = {
    config: {
        name: string
        runtime: {
            minecraft: string
            mod_loader_type: "Fabric" | "Quilt" | "Forge" | "Neoforge" | null
            mod_loader_version: string | null
        }
        group?: string
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
            ignore_patch_discrepanicies?: boolean
            extra_class_paths?: string
            gc?: "Serial" | "Parallel" | "ParallelOld" | "G1" | "Z"
            launcher_name?: string
            wrap_command?: string
            execute_before_launch?: string
            execute_after_launch?: string
        }
    }
    installed: boolean
}

type InstanceStore = {
    currentInstance: Instance
    instances: Instance[]
    // installProgress: {
    //     instanceName: string
    //     step: number
    //     completed: number
    //     total: number
    // }[]
    installProgress: Map<string, { step: number; completed: number; total: number }>
    launchedInstances: Map<
        string,
        {
            launchAt: Date
            running: number
        }
    >
}
export const useInstanceStore = defineStore("instance", {
    state: (): InstanceStore => {
        return {
            currentInstance: {
                config: {
                    name: "",
                    runtime: {
                        minecraft: "",
                        mod_loader_type: null,
                        mod_loader_version: "",
                    },
                    launch_config: {
                        enable_instance_specific_settings: false,
                    },
                },
                installed: true,
            },
            instances: [],
            installProgress: new Map(),
            launchedInstances: new Map(),
        }
    },
})
