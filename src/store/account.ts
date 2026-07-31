// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Accounts, listAccounts } from "@conic/account"
import { defineStore } from "pinia"

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
    },
})
