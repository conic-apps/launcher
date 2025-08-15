// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Instance, listInstances } from "@conic/instance"
import { defineStore } from "pinia"
import { useConfigStore } from "./config"

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
    actions: {
        async fetchInstances() {
            this.instances = await listInstances("Name")
            const foundCurrentInstance = this.instances.find((value) => {
                return value.id === this.currentInstance.id
            })
            if (foundCurrentInstance) {
                this.currentInstance = foundCurrentInstance
            } else {
                const config = useConfigStore()
                if (!config.accessibility.hide_latest_release) {
                    this.currentInstance = this.instances[0]
                } else if (!config.accessibility.hide_latest_snapshot) {
                    this.currentInstance = this.instances[0]
                } else {
                    this.currentInstance = this.instances[0]
                }
            }
        },
    },
})
