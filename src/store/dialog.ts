// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"
import { ref } from "vue"
import type { Account } from "@conic/account"
import { Instance } from "@conic/instance"

export const useDialogStore = defineStore("dialog", () => {
    const updateReminder = ref({
        visible: true,
        version: null as null | string,
        versionType: null as null | "release" | "snapshot",
    })
    const updateApp = ref({ visible: false })
    const accountManager = ref({ visible: false })
    const accountAdd = ref({ visible: false })
    const createInstance = ref({ visible: false })
    const confirmQuitApp = ref({ visible: false })
    const noAccountError = ref({ visible: false })
    const noMicrosoftAccountError = ref({ visible: false })
    const noSuitableJavaError = ref({ visible: false })
    const accountRefreshFailed = ref({ visible: false })
    const multiplayerExtension = ref({
        visible: false,
        currentComponent: "downloadDescription" as
            | "downloadDescription"
            | "downloadProgress"
            | "multiplayerManager",
        multiplayerManagerComponent: "waiting" as
            | "waiting"
            | "hostScan"
            | "hostReady"
            | "guestCodeInput"
            | "guestJoining"
            | "guestReady"
            | "exception",
    })
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
            has_background: false,
            last_played: 0,
        } as Instance,
    })
    const confirmDeleteAccount = ref({
        visible: false,
        account: null as Account | null,
    })
    const confirmDeleteSave = ref({
        visible: false,
        folderName: "",
        levelName: "",
    })
    const uploadSkin = ref({
        visible: false,
        accountType: "Microsoft" as Account["type"],
        skinUrl: "",
        capeUrl: "",
        textureType: "skin" as "skin" | "cape",
    })
    return {
        updateReminder,
        updateApp,
        accountManager,
        accountAdd,
        createInstance,
        confirmDeleteInstance,
        confirmDeleteAccount,
        confirmDeleteSave,
        confirmQuitApp,
        noAccountError,
        noMicrosoftAccountError,
        noSuitableJavaError,
        accountRefreshFailed,
        multiplayerExtension,
        uploadSkin,
    }
})
