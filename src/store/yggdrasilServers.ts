// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"
import { ref, watch } from "vue"
import { useAccountStore } from "./account"
import { getYggdrasilServerInfo, YggdrasilServerInfo } from "@conic/account"

export const useYggdrasilServersStore = defineStore("yggdrasil_servers", () => {
    const accountStore = useAccountStore()
    const serverList = ref<Record<string, YggdrasilServerInfo>>({})
    let refreshTimer: ReturnType<typeof setTimeout> | undefined
    async function refresh() {
        const apiRoots = [
            ...new Set(Object.values(accountStore.yggdrasil).map((account) => account.api_root)),
        ]
        const entries = await Promise.all(
            apiRoots.map(async (apiRoot) => {
                try {
                    const info = await getYggdrasilServerInfo(apiRoot)
                    return [apiRoot, info] as const
                } catch (error) {
                    console.debug(error)
                    return null
                }
            }),
        )
        serverList.value = Object.fromEntries(
            entries.filter((e): e is readonly [string, YggdrasilServerInfo] => e !== null),
        )
    }

    function scheduleRefresh() {
        clearTimeout(refreshTimer)

        refreshTimer = setTimeout(() => {
            void refresh()
        }, 100)
    }

    watch(() => accountStore.yggdrasil, scheduleRefresh, {
        deep: true,
        immediate: true,
    })

    return {
        serverList,
        refresh,
    }
})
