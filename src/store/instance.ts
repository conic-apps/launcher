// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { createInstance, getInstanceById, listInstances } from "@conic/instance"
import { defineStore } from "pinia"
import { useConfigStore } from "./config"
import { ref } from "vue"
import { getMinecrafVersionManifest } from "@conic/install"

const LATEST_RELEASE_INSTANCE_ID = "00000000-0000-0000-0000-00000001bf52"
const LATEST_SNAPSHOT_INSTANCE_ID = "00000000-0000-0000-0000-0000001d4b42"

async function ensureLatestInstancesExistance() {
    if (!(await getInstanceById(LATEST_RELEASE_INSTANCE_ID))) {
        const minecraftVersionManifest = await getMinecrafVersionManifest()
        await createInstance(
            {
                launch_config: { enable_instance_specific_settings: false },
                name: "Latest Release",
                runtime: { minecraft: minecraftVersionManifest.latest.release },
            },
            LATEST_RELEASE_INSTANCE_ID,
        )
    }

    if (!(await getInstanceById(LATEST_SNAPSHOT_INSTANCE_ID))) {
        const minecraftVersionManifest = await getMinecrafVersionManifest()
        await createInstance(
            {
                launch_config: { enable_instance_specific_settings: false },
                name: "Latest Snapshot",
                runtime: { minecraft: minecraftVersionManifest.latest.snapshot },
            },
            LATEST_SNAPSHOT_INSTANCE_ID,
        )
    }
}

await ensureLatestInstancesExistance()
const listedInstances = await listInstances("Name") // TODO: Error handling, show error dialog
console.log(listedInstances)

export const useInstanceStore = defineStore("instance", () => {
    const instances = ref(listedInstances)
    const currentInstance = ref(listedInstances[0])
    const launchedInstances = ref(new Map())
    async function fetchInstances() {
        await ensureLatestInstancesExistance()
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
