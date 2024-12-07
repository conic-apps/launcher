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
    }
    installed: boolean
}

type InstanceStore = {
    currentInstance: Instance
    instances: Instance[]
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
                },
                installed: true,
            },
            instances: [],
        }
    },
})
