// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { UpdateChannel } from "@conic/config"
import {
    cancelUpdate as cancelUpdateCommand,
    checkUpdate as checkUpdateCommand,
    downloadAndInstallUpdate as downloadAndInstallUpdateCommand,
    type UpdateInfo,
    type UpdateProgress,
} from "@conic/update"
import { defineStore } from "pinia"
import { ref } from "vue"

export const useUpdateStore = defineStore("update", () => {
    const checking = ref(false)
    const updating = ref(false)
    const updateInfo = ref<UpdateInfo | null>(null)
    const progress = ref<UpdateProgress>({ phase: "checking" })
    const error = ref("")

    async function check(channel: UpdateChannel) {
        checking.value = true
        error.value = ""
        updateInfo.value = null
        try {
            updateInfo.value = await checkUpdateCommand(channel)
        } catch (e) {
            error.value = String(e)
        } finally {
            checking.value = false
        }
    }

    async function downloadAndInstall(channel: UpdateChannel) {
        updating.value = true
        error.value = ""
        progress.value = { phase: "checking" }
        try {
            await downloadAndInstallUpdateCommand(channel, (nextProgress) => {
                progress.value = nextProgress
            })
        } catch (e) {
            error.value = String(e)
        } finally {
            updating.value = false
        }
    }

    function cancel() {
        void cancelUpdateCommand()
    }

    return { checking, updating, updateInfo, progress, error, check, downloadAndInstall, cancel }
})
