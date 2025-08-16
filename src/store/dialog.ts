// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"
import { ref } from "vue"

export const useDialogStore = defineStore("dialog", () => {
    const updateReminder = ref({ visible: false })
    const accountManager = ref({ visible: false })
    const logViewer = ref({ visible: false })
    const createInstance = ref({ visible: false })
    const confirmDeleteInstance = ref({
        visible: false,
        instanceToDelete: {
            config: {
                name: "Unknown",
                runtime: {
                    minecraft: "1.18.2",
                },
                launch_config: {
                    enable_instance_specific_settings: false,
                },
            },
            installed: false,
            id: "00000000-0000-0000-0000-000000000000",
        },
    })
    return { updateReminder, accountManager, logViewer, createInstance, confirmDeleteInstance }
})
