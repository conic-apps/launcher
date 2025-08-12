// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

invoke("plugin:account|cmd_test").then((res) => {
    console.log(res)
})

export type Accounts = {
    microsoft: MicrosoftAccount[]
    offline: OfflineAccount[]
    authlib_injector: Map<string, AuthlibInjectorAccount>
}

export enum AccountType {
    Microsoft = "Microsoft",
    Offline = "Offline",
    AuthlibInjector = "AuthlibInjector",
}

export type MicrosoftAccount = {
    refresh_token?: string
    access_token?: string
    expires_on?: number
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

export async function getMicrosoftAccount(uuid: string): Promise<MicrosoftAccount[]> {
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

export type OfflineAccount = {
    name: string
    uuid: string
    skin?: string
}

export async function addOfflineAccount(name: string) {
    await invoke("plugin:account|cmd_add_offline_account", { name })
}

export async function deleteOfflineAccount(uuid: string) {
    await invoke("plugin:account|cmd_delete_offline_account", { uuid })
}

export async function updateOfflineAccount(account: OfflineAccount) {
    await invoke("plugin:account|cmd_update_offline_account", { account })
}

export async function getOfflineAccount(uuid: string): Promise<OfflineAccount[]> {
    return await invoke("plugin:account|cmd_get_offline_account", { uuid })
}

export async function addYggdrasilServer(apiRoot: string) {
    await invoke("plugin:account|cmd_add_yggdrasil_server", { apiRoot })
}

export async function deleteYggdrasilServer(indexToDelete: number) {
    await invoke("plugin:account|cmd_delete_yggdrasil_server", { indexToDelete })
}

export async function listYggdrasilServer(): Promise<string[]> {
    return await invoke("plugin:account|cmd_list_yggdrasil_server")
}

export async function getYggdrasilServerInfo(apiRoot: string): Promise<YggdrasilServerInfo> {
    return await invoke("plugin:account|cmd_get_yggdrasil_server_info", { apiRoot })
}

export type YggdrasilServerInfo = {
    // eslint-disable-next-line
    meta: NonNullable<any>
    skinDomains: string[]
    signaturePublicKey: string
}

export async function yggdrasilLogin(
    apiRoot: string,
    username: string,
    password: string,
): Promise<LoginResponse> {
    return await invoke("plugin:account|cmd_yggdrasil_login", { apiRoot, username, password })
}

export type LoginResponse = {
    accessToken: string
    clientToken: string
    availableProfiles: {
        id: string
        name: string
    }[]
    selectedProfile?: {
        id: string
        name: string
    }
}

export async function addAuthlibAccount(account: AuthlibInjectorAccount) {
    await invoke("plugin:account|cmd_add_authlib_account", { account })
}

export type AuthlibInjectorAccount = {
    api_root: string
    account_identifier: string
    access_token: string
    client_token: string
    profile_name: string
    profile_uuid: string
    added_at: number
}

export async function deleteAuthlibAccount(accountKey: string) {
    await invoke("plugin:account|cmd_delete_authlib_account", { accountKey })
}

export async function getAuthlibProfileInfo(apiRoot: string, uuid: string): Promise<Profile> {
    return await invoke("plugin:account|cmd_get_authlib_profile_info", { apiRoot, uuid })
}

export type Profile = {
    id: string
    name: string
    properties: {
        name: string
        value: string
    }[]
}

export async function getAuthlibAccount(accountKey: string): Promise<AuthlibInjectorAccount> {
    return await invoke("plugin:account|cmd_get_authlib_account", { accountKey })
}

export async function reloginAccount(uuid: string, accountType: AccountType, credential: string) {
    await invoke("plugin:account|cmd_relogin_account", { uuid, accountType, credential })
}

export async function getAvatar(src: string, size: number): Promise<string> {
    const canvas = document.createElement("canvas")
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext("2d")
    if (ctx == null) {
        return ""
    }
    const img = new Image()
    img.src = src
    await new Promise<void>((reslove) => {
        img.onload = function () {
            const scale = img.width / 64
            const faceOffset = Math.round(size / 18.0)
            ctx.imageSmoothingEnabled = false
            /* Inspired by HMCL */
            ctx.drawImage(
                img,
                8 * scale,
                8 * scale,
                16 * scale - 8 * scale,
                16 * scale - 8 * scale,
                faceOffset,
                faceOffset,
                size - faceOffset - faceOffset,
                size - faceOffset - faceOffset,
            )
            ctx.drawImage(
                img,
                40 * scale,
                8 * scale,
                48 * scale - 40 * scale,
                16 * scale - 8 * scale,
                0,
                0,
                size,
                size,
            )
            reslove()
        }
    })
    return canvas.toDataURL("image/png")
}
