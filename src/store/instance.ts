// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Instance } from "@conic/instance"
import { defineStore } from "pinia"

type InstanceStore = {
    currentInstance: Instance
    instances: Instance[]
    // installProgress: {
    //     instanceName: string
    //     step: number
    //     completed: number
    //     total: number
    // }[]
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
                        mod_loader_type: undefined,
                        mod_loader_version: undefined,
                    },
                    launch_config: {
                        enable_instance_specific_settings: false,
                    },
                },
                installed: true,
                id: "",
            },
            instances: [],
            launchedInstances: new Map(),
        }
    },
})
