// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import {
    createInstance,
    getInstanceById,
    LATEST_RELEASE_INSTANCE_ID,
    LATEST_SNAPSHOT_INSTANCE_ID,
    listInstances,
    type InstanceSort,
} from "@conic/instance"
import { defineStore } from "pinia"
import { useConfigStore } from "./config"
import { ref, watch } from "vue"
import { getMinecrafVersionManifest } from "@conic/install"

async function ensureLatestInstancesExistance() {
    const promises = []
    if (!(await getInstanceById(LATEST_RELEASE_INSTANCE_ID))) {
        const minecraftVersionManifest = await getMinecrafVersionManifest()
        promises.push(
            createInstance(
                {
                    launch_config: { enable_instance_specific_settings: false },
                    name: "Latest Release",
                    runtime: { minecraft: minecraftVersionManifest.latest.release },
                },
                LATEST_RELEASE_INSTANCE_ID,
            ),
        )
    }
    if (!(await getInstanceById(LATEST_SNAPSHOT_INSTANCE_ID))) {
        const minecraftVersionManifest = await getMinecrafVersionManifest()
        promises.push(
            createInstance(
                {
                    launch_config: { enable_instance_specific_settings: false },
                    name: "Latest Snapshot",
                    runtime: { minecraft: minecraftVersionManifest.latest.snapshot },
                },
                LATEST_SNAPSHOT_INSTANCE_ID,
            ),
        )
    }
    await Promise.all(promises)
}

await ensureLatestInstancesExistance()
const sort = ref<InstanceSort>("Playtime")
const listedInstances = await listInstances(sort.value) // TODO: Error handling, show error dialog

export const useInstanceStore = defineStore("instance", () => {
    const instances = ref(listedInstances)

    const currentInstanceId = localStorage.getItem("currentInstanceId")
    const currentInstance = ref(
        currentInstanceId
            ? (listedInstances.find((instance) => instance.id === currentInstanceId) ??
                  listedInstances[0])
            : listedInstances[0],
    )
    watch(currentInstance, (instance) => localStorage.setItem("currentInstanceId", instance.id))

    const launchedInstances = ref(new Map())

    let loadToken = 0

    async function loadInstances() {
        const token = ++loadToken
        await ensureLatestInstancesExistance()
        const loadedInstances = await listInstances(sort.value)
        if (token !== loadToken) return
        instances.value = loadedInstances
        ensureCurrentInstanceAvailable()
    }

    function setSort(sortBy: InstanceSort) {
        if (sort.value === sortBy) return
        sort.value = sortBy
        void loadInstances()
    }

    function ensureCurrentInstanceAvailable() {
        const foundCurrentInstance = instances.value.find(
            (value) => value.id === currentInstance.value.id,
        )
        if (foundCurrentInstance) {
            currentInstance.value = foundCurrentInstance
        } else {
            const config = useConfigStore()
            if (!config.accessibility.hide_latest_release) {
                currentInstance.value =
                    instances.value.find(
                        (instance) => instance.id === LATEST_RELEASE_INSTANCE_ID,
                    ) ?? instances.value[0]
            } else if (!config.accessibility.hide_latest_snapshot) {
                currentInstance.value =
                    instances.value.find(
                        (instances) => instances.id === LATEST_SNAPSHOT_INSTANCE_ID,
                    ) ?? instances.value[0]
            } else {
                currentInstance.value = instances.value[0]
            }
        }
    }
    return {
        instances,
        sort,
        currentInstanceId,
        currentInstance,
        launchedInstances,
        loadInstances,
        setSort,
        ensureCurrentInstanceAvailable,
    }
})
