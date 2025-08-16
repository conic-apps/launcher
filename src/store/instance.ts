// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { listInstances } from "@conic/instance"
import { defineStore } from "pinia"
import { useConfigStore } from "./config"
import { ref } from "vue"

const listedInstances = await listInstances("Name")

export const useInstanceStore = defineStore("instance", () => {
    const instances = ref(listedInstances)
    const currentInstance = ref(listedInstances[0])
    const launchedInstances = ref(new Map())
    async function fetchInstances() {
        instances.value = await listInstances("Name")
        ensureCurrentInstanceAvailable()
    }
    function ensureCurrentInstanceAvailable() {
        const foundCurrentInstance = instances.value.find((value) => {
            return value.id === currentInstance.value.id
        })
        if (foundCurrentInstance) {
            currentInstance.value = foundCurrentInstance
        } else {
            const config = useConfigStore()
            if (!config.accessibility.hide_latest_release) {
                currentInstance.value = instances.value[0]
            } else if (!config.accessibility.hide_latest_snapshot) {
                currentInstance.value = instances.value[0]
            } else {
                currentInstance.value = instances.value[0]
            }
        }
    }
    return {
        instances,
        currentInstance,
        launchedInstances,
        fetchInstances,
        ensureCurrentInstanceAvailable,
    }
})
