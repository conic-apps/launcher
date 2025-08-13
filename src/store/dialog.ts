// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Instance } from "@conic/instance"
import { defineStore } from "pinia"

export type DialogStore = {
    updateReminder: {
        visible: boolean
        // NOTE: You cam add other option here
    }
    accountManager: { visible: boolean }
    logViewer: { visible: boolean }
    createInstance: { visible: boolean }
    confirmDeleteInstance: {
        visible: boolean
        instanceToDelete: Instance
    }
}

// const config = (await invoke("read_config_file")) as Config
export const useDialogStore = defineStore("global_config", {
    state: (): DialogStore => {
        return {
            updateReminder: {
                visible: false,
                // NOTE: You cam add other option here
            },
            accountManager: { visible: false },
            logViewer: { visible: false },
            createInstance: { visible: false },
            confirmDeleteInstance: {
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
            },
        }
    },
})
