// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

type Accounts = {
    microsoft: MicrosoftAccount[]
}

export type MicrosoftAccount = {
    refresh_token?: string
    access_token?: string
    token_deadline?: number
    profile: {
        avatar: string
        profile_name: string
        uuid: string
        skins: {
            id: string
            state: string
            textureKey: string
            url: string
            variant: string
        }[]
        capes: {
            alias: string
            id: string
            state: string
            url: string
        }[]
    }
    account_type: "Microsoft" | "Offline"
}

export async function listAccounts(): Promise<Accounts> {
    return await invoke("plugin:account|cmd_list_accounts")
}

export async function getMicrosoftAccount(uuid: string): Promise<MicrosoftAccount> {
    return await invoke("plugin:account|cmd_get_microsoft_account", { uuid })
}

export async function deleteMicrosoftAccount(uuid: string) {
    return await invoke("plugin:account|cmd_delete_microsoft_account", { uuid })
}

export async function addMicrosoftAccount(code: string) {
    return await invoke("plugin:account|cmd_add_microsoft_account", { code })
}

export async function refreshAllMicrosoftAccounts() {
    await invoke("plugin:account|cmd_refresh_all_microsoft_accounts")
}

export async function refreshMicrosoftAccount(uuid: string) {
    await invoke("plugin:account|cmd_refresh_microsoft_account", { uuid })
}

export async function addOfflineAccount(name: string) {
    await invoke("plugin:account|cmd_add_offline_account", { name })
}
