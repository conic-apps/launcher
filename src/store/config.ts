// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { loadConfigFile, saveConfigToFile } from "@conic/config"
import { defineStore } from "pinia"
import { ref, toRefs, watch } from "vue"

const initialConfig = await loadConfigFile()

export const useConfigStore = defineStore("global_config", () => {
    const config = ref(initialConfig)
    let saveQueue = Promise.resolve()
    watch(
        config,
        () => {
            saveQueue = saveQueue.catch(() => {}).then(() => saveConfigToFile(config.value)) // TODO: Debounce
        },
        {
            deep: true,
        },
    )
    return {
        ...toRefs(config.value),
    }
})
