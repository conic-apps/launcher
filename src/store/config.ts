// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { getSystemLanguage, loadConfigFile, saveConfigToFile } from "@conic/config"
import { defineStore } from "pinia"
import { ref, toRefs, watch } from "vue"

const initialConfig = await loadConfigFile()
initialConfig.language = initialConfig.language ?? (await getSystemLanguage())
console.log(initialConfig)

export const useConfigStore = defineStore("global_config", () => {
    const config = ref(initialConfig)
    let saveQueue = Promise.resolve()
    watch(
        config,
        () => {
            saveQueue = saveQueue.catch(() => {}).then(() => saveConfigToFile(config.value))
        },
        {
            deep: true,
        },
    )
    return {
        ...toRefs(config.value),
    }
})
