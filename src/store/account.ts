// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Accounts, listAccounts } from "@conic/account"
import { defineStore } from "pinia"
import { useConfigStore } from "./config"

let accounts: Accounts
try {
    accounts = await listAccounts()
} catch (error) {
    console.error(error)
    accounts = {
        microsoft: [],
        offline: [],
        yggdrasil: [],
    }
}

export const useAccountStore = defineStore("accounts", {
    state: (): Accounts => {
        return accounts
    },
    actions: {
        async reloadFromFile() {
            const accounts = await listAccounts()
            this.microsoft = accounts.microsoft
            this.yggdrasil = accounts.yggdrasil
            this.offline = accounts.offline
        },
        selectNextAccount() {
            const configStore = useConfigStore()
            if (this.microsoft.length > 0) {
                configStore.current_account = { type: "Microsoft", data: this.microsoft[0] }
            } else if (this.offline.length > 0) {
                configStore.current_account = { type: "Offline", data: this.offline[0] }
            } else if (Object.values(this.yggdrasil).length > 0) {
                const yggdrasilAccounts = Object.values(this.yggdrasil)
                configStore.current_account = { type: "Yggdrasil", data: yggdrasilAccounts[0] }
            } else {
                configStore.current_account = null
            }
        },
    },
})
