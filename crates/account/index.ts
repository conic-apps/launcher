// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type AccountError =
    | { kind: "Io"; message: string }
    | { kind: "UrlParse"; message: string }
    | { kind: "JsonParse"; message: string }
    | { kind: "ToStr"; message: string }
    | { kind: "Network"; message: string }
    | { kind: "AccountNotfound"; message: string }
    | { kind: "ProfileUnavailable"; message: string }
    | { kind: "MicrosoftResponseMissingKey"; message: string }
    | { kind: "InvalidALIResponse"; message: string }

export type AccountType = "Microsoft" | "Offline" | "Yggdrasil"

export type Accounts = {
    microsoft: MicrosoftAccount[]
    offline: OfflineAccount[]
    third_party_yggdrasil: Record<string, YggdrasilAccount>
}

export type MicrosoftAccount = {
    refresh_token: string
    minecraft_access_token: string
    expires_at: number
    profile: {
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
}

export type OfflineAccount = {
    name: string
    uuid: string
    skin?: string
}

export type YggdrasilAccount = {
    api_root: string
    account_identifier: string
    access_token: string
    client_token: string
    profile_name: string
    profile_uuid: string
    added_at: number
}

export type YggdrasilServerInfo = {
    meta: Record<string, unknown>
    skinDomains: string[]
    signaturePublicKey: string
}

export type DeviceCodeResponse = {
    user_code: string
    device_code: string
    verification_uri: string
    expires_in: number
    interval: number
    message: string
}

export type DeviceCodePollResult = {
    status:
        | "success"
        | "authorization_pending"
        | "authorization_declined"
        | "bad_verification_code"
        | "expired_token"
        | "slow_down"
    access_token: string | null
    refresh_token: string | null
    expires_in: number | null
}

export type RedeemAccessTokenResult = {
    access_token: string
    refresh_token: string
}

export type AuthResponse = {
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

export type YggdrasilProfile = {
    id: string
    name: string
    properties: {
        name: string
        value: string
    }[]
}

export async function listAccounts(): Promise<Accounts> {
    return await invoke("plugin:account|cmd_list_accounts")
}

export async function getMicrosoftAccount(uuid: string): Promise<MicrosoftAccount> {
    return await invoke("plugin:account|cmd_get_microsoft_account", { uuid })
}

export async function deleteMicrosoftAccount(uuid: string) {
    await invoke("plugin:account|cmd_delete_microsoft_account", { uuid })
}

export async function addMicrosoftAccount(account: MicrosoftAccount) {
    await invoke("plugin:account|cmd_add_microsoft_account", { account })
}

export async function updateMicrosoftAccount(uuid: string, account: MicrosoftAccount) {
    await invoke("plugin:account|cmd_update_microsoft_account", { uuid, account })
}

export async function redeemAccessToken(code: string): Promise<RedeemAccessTokenResult> {
    return await invoke("plugin:account|cmd_redeem_access_token", { code })
}

export async function microsoftAccessTokenAuthFlow(
    accessToken: string,
    refreshToken: string,
): Promise<MicrosoftAccount> {
    return await invoke("plugin:account|cmd_microsoft_access_token_auth_flow", {
        accessToken,
        refreshToken,
    })
}

export async function requestDeviceCode(): Promise<DeviceCodeResponse> {
    return await invoke("plugin:account|cmd_request_device_code")
}

export async function pollDeviceCode(deviceCode: string): Promise<DeviceCodePollResult> {
    return await invoke("plugin:account|cmd_poll_device_code", { deviceCode })
}

export async function refreshMicrosoftAccount(
    uuid: string,
    forceRefresh: boolean,
): Promise<MicrosoftAccount> {
    return await invoke("plugin:account|cmd_refresh_microsoft_account", { uuid, forceRefresh })
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

export async function getOfflineAccount(uuid: string): Promise<OfflineAccount> {
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

export async function yggdrasilAuthenticateAccount(
    apiRoot: string,
    username: string,
    password: string,
): Promise<AuthResponse> {
    return await invoke("plugin:account|cmd_yggdrasil_authenticate_account", {
        apiRoot,
        username,
        password,
    })
}

export async function yggdrasilValidateAccount(account: YggdrasilAccount): Promise<boolean> {
    return await invoke("plugin:account|cmd_yggdrasil_validate_account", { account })
}

export async function yggdrasilRefreshAccount(account: YggdrasilAccount): Promise<YggdrasilAccount> {
    return await invoke("plugin:account|cmd_yggdrasil_refresh_account", { account })
}

export async function yggdrasilInvalidateAccount(
    apiRoot: string,
    accessToken: string,
    clientToken: string,
) {
    await invoke("plugin:account|cmd_yggdrasil_invalidate_account", { apiRoot, accessToken, clientToken })
}

export async function yggdrasilGetProfile(
    apiRoot: string,
    uuid: string,
): Promise<YggdrasilProfile> {
    return await invoke("plugin:account|cmd_yggdrasil_get_profile", { apiRoot, uuid })
}

export async function addYggdrasilAccount(account: YggdrasilAccount) {
    await invoke("plugin:account|cmd_add_yggdrasil_account", { account })
}

export async function deleteYggdrasilAccount(accountKey: string) {
    await invoke("plugin:account|cmd_delete_yggdrasil_account", { accountKey })
}

export async function getYggdrasilAccount(accountKey: string): Promise<YggdrasilAccount> {
    return await invoke("plugin:account|cmd_get_yggdrasil_account", { accountKey })
}

export async function listYggdrasilAccounts(): Promise<Record<string, YggdrasilAccount>> {
    return await invoke("plugin:account|cmd_list_yggdrasil_accounts")
}

export async function updateYggdrasilAccount(accountKey: string, account: YggdrasilAccount) {
    await invoke("plugin:account|cmd_update_yggdrasil_account", { accountKey, account })
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
    await new Promise<void>((resolve) => {
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
            resolve()
        }
    })
    return canvas.toDataURL("image/png")
}
