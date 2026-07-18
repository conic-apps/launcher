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
}

export const useAccountStore = defineStore("accounts", {
    state: (): Accounts => {
        return accounts
    },
})
