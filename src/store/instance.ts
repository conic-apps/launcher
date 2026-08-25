// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Instance, listInstances, type InstanceSort } from "@conic/instance"
import { defineStore } from "pinia"
import { ref, watch } from "vue"

const sort = ref<InstanceSort>("Playtime")
const listedInstances = await listInstances(sort.value)

export const useInstanceStore = defineStore("instance", () => {
    const instances = ref(listedInstances)

    const currentInstanceId = localStorage.getItem("currentInstanceId")
    const currentInstance = ref<Instance | null>(
        currentInstanceId
            ? (listedInstances.find((instance) => instance.id === currentInstanceId) ??
                  listedInstances[0])
            : listedInstances[0],
    )
    watch(currentInstance, (instance) =>
        localStorage.setItem("currentInstanceId", instance?.id ?? ""),
    )

    const launchedInstances = ref(new Map())

    let loadToken = 0

    async function loadInstances() {
        const token = ++loadToken
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
            (value) => value.id === currentInstance.value?.id,
        )
        if (foundCurrentInstance) {
            currentInstance.value = foundCurrentInstance
        } else {
            currentInstance.value = instances.value[0]
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
