// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"

// const config = (await invoke("read_config_file")) as Config
export const useDialogStore = defineStore("global_config", {
    state: () => {
        return {
            updateReminder: false,
            accountManager: false,
            logViewer: false,
            createInstance: false,
            confirmDeleteInstance: false,
        }
    },
})
