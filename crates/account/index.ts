// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"
import md5 from "blueimp-md5"

export type AccountError =
    | { kind: "Io"; message: string }
    | { kind: "UrlParse"; message: string }
    | { kind: "InvalidBaseUrl"; message: string }
    | { kind: "JsonParse"; message: string }
    | { kind: "ToStr"; message: string }
    | { kind: "Network"; message: string }
    | { kind: "AccountNotfound"; message: string }
    | { kind: "ProfileUnavailable"; message: string }
    | { kind: "MicrosoftResponseMissingKey"; message: string }
    | { kind: "InvalidALIResponse"; message: string }
    | { kind: "YggdrasilTextureParseError"; message: string }
    | { kind: "Base64DecodeError"; message: string }

export type Accounts = {
    microsoft: MicrosoftAccount[]
    offline: OfflineAccount[]
    yggdrasil: YggdrasilAccount[]
}

export type Account =
    | {
          type: "Microsoft"
          data: MicrosoftAccount
      }
    | {
          type: "Offline"
          data: OfflineAccount
      }
    | {
          type: "Yggdrasil"
          data: YggdrasilAccount
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
    username: string
    access_token: string
    client_token: string
    identifier: string
    profile: YggdrasilProfile
    textures: Record<string, YggdrasilTexture>
    added_at: number
}

export type YggdrasilServerInfo = {
    meta: Record<string, unknown>
    skinDomains: string[]
    signaturePublickey: string
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
    availableProfiles: YggdrasilProfile[]
    selectedProfile?: YggdrasilProfile
}

export type YggdrasilTexture = {
    url: string
    metadata: Record<string, string> | null
}

export type YggdrasilProfile = {
    id: string
    name: string
    properties:
        | {
              name: string
              value: string
              signature?: string
          }[]
        | null
}

export async function listAccounts(): Promise<Accounts> {
    return await invoke("plugin:account|cmd_list_accounts")
}

export async function saveSkin(base64SkinUrl: string, path: string): Promise<Accounts> {
    return await invoke("plugin:account|cmd_save_skin", { base64SkinUrl, path })
}

export async function getMicrosoftAccount(uuid: string): Promise<MicrosoftAccount> {
    return await invoke("plugin:account|cmd_microsoft_get_account", { uuid })
}

export async function deleteMicrosoftAccount(uuid: string) {
    await invoke("plugin:account|cmd_microsoft_delete_account", { uuid })
}

export async function addMicrosoftAccount(account: MicrosoftAccount) {
    await invoke("plugin:account|cmd_microsoft_add_account", { account })
}

export async function updateMicrosoftAccount(uuid: string, account: MicrosoftAccount) {
    await invoke("plugin:account|cmd_microsoft_update_account", { uuid, account })
}

export async function redeemAccessToken(code: string): Promise<RedeemAccessTokenResult> {
    return await invoke("plugin:account|cmd_microsoft_redeem_access_token", { code })
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
    return await invoke("plugin:account|cmd_microsoft_request_device_code")
}

export async function pollDeviceCode(deviceCode: string): Promise<DeviceCodePollResult> {
    return await invoke("plugin:account|cmd_microsoft_poll_device_code", { deviceCode })
}

export async function refreshMicrosoftAccount(
    uuid: string,
    forceRefresh: boolean,
): Promise<MicrosoftAccount> {
    return await invoke("plugin:account|cmd_microsoft_refresh_account", { uuid, forceRefresh })
}

export async function addOfflineAccount(name: string, uuid: string) {
    await invoke("plugin:account|cmd_offline_add_account", { name, uuid })
}

export async function deleteOfflineAccount(uuid: string) {
    await invoke("plugin:account|cmd_offline_delete_account", { uuid })
}

export async function updateOfflineAccount(account: OfflineAccount) {
    await invoke("plugin:account|cmd_offline_update_account", { account })
}

export async function getOfflineAccount(uuid: string): Promise<OfflineAccount> {
    return await invoke("plugin:account|cmd_offline_get_account", { uuid })
}

export async function getYggdrasilServerInfo(apiRoot: string): Promise<YggdrasilServerInfo> {
    return await invoke("plugin:account|cmd_yggdrasil_get_server_info", { apiRoot })
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

export async function yggdrasilRefreshAccount(
    account: YggdrasilAccount,
): Promise<YggdrasilAccount> {
    return await invoke("plugin:account|cmd_yggdrasil_refresh_account", { account })
}

export async function yggdrasilInvalidateAccount(
    apiRoot: string,
    accessToken: string,
    clientToken: string,
) {
    await invoke("plugin:account|cmd_yggdrasil_invalidate_account", {
        apiRoot,
        accessToken,
        clientToken,
    })
}

export async function yggdrasilGetProfile(
    apiRoot: string,
    uuid: string,
): Promise<YggdrasilProfile> {
    return await invoke("plugin:account|cmd_yggdrasil_get_profile", { apiRoot, uuid })
}

export async function yggdrasilGetProfiles(
    apiRoot: string,
    uuids: string[],
): Promise<YggdrasilProfile[]> {
    return await invoke("plugin:account|cmd_yggdrasil_get_profiles", { apiRoot, uuids })
}

export function yggdrasilGetSkinUrl(profile: YggdrasilProfile): string | undefined {
    const texturesProp = profile.properties?.find((p) => p.name === "textures")
    if (!texturesProp) return undefined
    try {
        const decoded = JSON.parse(atob(texturesProp.value))
        return decoded.textures?.SKIN?.url as string | undefined
    } catch {
        return undefined
    }
}

export function yggdrasilGetCapeUrl(profile: YggdrasilProfile): string | undefined {
    const texturesProp = profile.properties?.find((p) => p.name === "textures")
    if (!texturesProp) return undefined
    try {
        const decoded = JSON.parse(atob(texturesProp.value))
        return decoded.textures?.CAPE?.url as string | undefined
    } catch {
        return undefined
    }
}

export async function addYggdrasilAccount(account: YggdrasilAccount) {
    await invoke("plugin:account|cmd_yggdrasil_add_account", { account })
}

export async function deleteYggdrasilAccount(account: YggdrasilAccount) {
    await invoke("plugin:account|cmd_yggdrasil_delete_account", { account })
}

export async function getYggdrasilAccount(accountKey: string): Promise<YggdrasilAccount> {
    return await invoke("plugin:account|cmd_yggdrasil_get_account", { accountKey })
}

export async function listYggdrasilAccounts(): Promise<YggdrasilAccount[]> {
    return await invoke("plugin:account|cmd_yggdrasil_list_accounts")
}

export async function updateYggdrasilAccount(accountKey: string, account: YggdrasilAccount) {
    await invoke("plugin:account|cmd_yggdrasil_update_account", { accountKey, account })
}

export async function getAvatarFromUrl(src: string, size: number): Promise<string> {
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

export function getUuidFromUsername(username: string): string {
    const bytes = getMd5Bytes(`OfflinePlayer:${username}`)
    bytes[6] = (bytes[6] & 0x0f) | 0x30
    bytes[8] = (bytes[8] & 0x3f) | 0x80
    return formatUuid(bytes)
}

function getMd5Bytes(string: string): Uint8Array {
    const hash = md5(string)
    const bytes = new Uint8Array(16)
    for (let i = 0; i < 16; i++) {
        bytes[i] = Number.parseInt(hash.slice(i * 2, i * 2 + 2), 16)
    }
    return bytes
}

function formatUuid(bytes: Uint8Array): string {
    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("")
    return [
        hex.slice(0, 8),
        hex.slice(8, 12),
        hex.slice(12, 16),
        hex.slice(16, 20),
        hex.slice(20, 32),
    ].join("-")
}

export type DefaultSkin = {
    textureName: "alex" | "ari" | "efe" | "kai" | "makena" | "noor" | "steve" | "sunny" | "zuri"
    modelType: "slim" | "wide"
}

const DEFAULT_SKINS: DefaultSkin[] = [
    { textureName: "alex", modelType: "slim" },
    { textureName: "ari", modelType: "slim" },
    { textureName: "efe", modelType: "slim" },
    { textureName: "kai", modelType: "slim" },
    { textureName: "makena", modelType: "slim" },
    { textureName: "noor", modelType: "slim" },
    { textureName: "steve", modelType: "slim" },
    { textureName: "sunny", modelType: "slim" },
    { textureName: "zuri", modelType: "slim" },
    { textureName: "alex", modelType: "wide" },
    { textureName: "ari", modelType: "wide" },
    { textureName: "efe", modelType: "wide" },
    { textureName: "kai", modelType: "wide" },
    { textureName: "makena", modelType: "wide" },
    { textureName: "noor", modelType: "wide" },
    { textureName: "steve", modelType: "wide" },
    { textureName: "sunny", modelType: "wide" },
    { textureName: "zuri", modelType: "wide" },
]

function uuidToLongs(uuid: string): [bigint, bigint] {
    const hex = uuid.replace(/-/g, "")

    const mostSigBits = BigInt("0x" + hex.slice(0, 16))
    const leastSigBits = BigInt("0x" + hex.slice(16, 32))

    return [mostSigBits, leastSigBits]
}

function uuidHashCode(uuid: string): number {
    const [mostSigBits, leastSigBits] = uuidToLongs(uuid)

    const hilo = mostSigBits ^ leastSigBits

    const high = Number((hilo >> 32n) & 0xffffffffn)
    const low = Number(hilo & 0xffffffffn)

    let hash = (high ^ low) >>> 0

    if (hash >= 0x80000000) {
        hash -= 0x100000000
    }

    return hash
}

function floorMod(a: number, b: number): number {
    return ((a % b) + b) % b
}

export function getDefaultSkin(uuid: string): DefaultSkin {
    const idx = floorMod(uuidHashCode(uuid), DEFAULT_SKINS.length)
    return DEFAULT_SKINS[idx]
}
