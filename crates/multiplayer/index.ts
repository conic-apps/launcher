// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { DownloadState } from "@conic/download"
import { Channel, invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export enum ConicNexusErrorKind {
    Io = "Io",
    ToStr = "ToStr",
    Network = "Network",
    AllSourceFailed = "AllSourceFailed",
    LibLoader = "LibLoader",
    ChecksumMismatch = "ChecksumMismatch",
    Aborted = "Aborted",
    ConicNexus = "ConicNexus",
}

export class ConicNexusLibraryDownloadTask {
    private _callbacks?: {
        onProgress?: (progress: DownloadState) => void
    }
    constructor(callbacks?: typeof this._callbacks) {
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<DownloadState>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
        }
        await invoke("plugin:multiplayer|cmd_spawn_download_library_task", {
            channel,
        })
    }
    async cancel() {
        await invoke("plugin:multiplayer|cmd_cancel_download_library_task")
    }
}

export async function isLibraryValid(): Promise<boolean> {
    try {
        await invoke("plugin:multiplayer|cmd_check_library_valid")
        return true
    } catch {
        return false
    }
}

export type MultiplayerState =
    | "waiting"
    | "host-scanning"
    | "host-starting"
    | "host-ok"
    | "guest-connecting"
    | "guest-starting"
    | "guest-ok"
    | "exception"
    | "unknown"

const STATE_NAMES = [
    "waiting",
    "host-scanning",
    "host-starting",
    "host-ok",
    "guest-connecting",
    "guest-starting",
    "guest-ok",
    "exception",
] as const

export function toStateName(state: number | string): MultiplayerState {
    if (typeof state === "number") {
        return STATE_NAMES[state] ?? "unknown"
    }
    return state as MultiplayerState
}

export type PlayerProfile = {
    machine_id: string
    name: string
    vendor: string
    kind: "HOST" | "LOCAL" | "GUEST"
}

export type OverlayInfo = {
    pid: number
    alive: boolean
    rpc_port: number
}

export type SessionStateDetail =
    | { port: number; profiles: PlayerProfile[]; overlay: OverlayInfo | null }
    | { url: string; profiles: PlayerProfile[]; overlay: OverlayInfo | null }
    | { error: { code: number; message: string } }
    | Record<string, never>

export type SessionState = {
    version: number
    state: MultiplayerState
    room_code: string
    detail: SessionStateDetail
}

export type PeerInfo = {
    hostname: string
    ipv4: string
    is_local: boolean
    nat: number
}

export type MultiplayerEventName =
    | "state-changed"
    | "player-joined"
    | "player-left"
    | "host-ready"
    | "guest-ready"
    | "fault"

export const MULTIPLAYER_EVENT_TYPES = {
    "state-changed": 0,
    "player-joined": 1,
    "player-left": 2,
    "host-ready": 3,
    "guest-ready": 4,
    fault: 5,
} as const

const TYPE_TO_NAME = Object.fromEntries(
    Object.entries(MULTIPLAYER_EVENT_TYPES).map(([name, type]) => [type, name]),
) as Record<number, MultiplayerEventName>

export type MultiplayerEventMap = {
    "state-changed": { state: number | MultiplayerState; version: number }
    "player-joined": { profile: PlayerProfile }
    "player-left": { machine_id: string }
    "host-ready": { room: string; port: number }
    "guest-ready": { url: string }
    fault: { code: number; message: string }
}

let eventChannelUnlisten: UnlistenFn | null = null
const eventHandlers = new Map<MultiplayerEventName, ((payload: unknown) => void)[]>()

async function ensureEventChannel() {
    if (eventChannelUnlisten) return
    eventChannelUnlisten = await listen<{ sequence: number; type: number; payload: unknown }>(
        "conic-nexus://event",
        (event) => {
            const name = TYPE_TO_NAME[event.payload.type]
            const callbacks = name ? eventHandlers.get(name) : undefined
            if (!callbacks) return
            for (const callback of callbacks) {
                callback(event.payload.payload)
            }
        },
    )
}

export async function on<K extends MultiplayerEventName>(
    name: K,
    callback: (payload: MultiplayerEventMap[K]) => void,
): Promise<() => void> {
    await ensureEventChannel()
    const callbacks = eventHandlers.get(name) ?? []
    callbacks.push(callback as (payload: unknown) => void)
    eventHandlers.set(name, callbacks)
    return () => off(name, callback as (payload: unknown) => void)
}

export async function off<K extends MultiplayerEventName>(
    name: K,
    callback: (payload: MultiplayerEventMap[K]) => void,
): Promise<void> {
    const callbacks = eventHandlers.get(name)
    if (!callbacks) return
    const index = callbacks.indexOf(callback as (payload: unknown) => void)
    if (index >= 0) callbacks.splice(index, 1)
    if (callbacks.length === 0) eventHandlers.delete(name)
}

export type CreateRoomOptions = {
    playerName?: string
    roomCode?: string
}

export async function createRoom(options: CreateRoomOptions = {}): Promise<void> {
    await invoke("plugin:multiplayer|cmd_create_room", {
        playerName: options.playerName,
        roomCode: options.roomCode,
    })
}

export type JoinRoomOptions = {
    roomCode: string
    playerName?: string
}

export async function joinRoom(options: JoinRoomOptions): Promise<void> {
    await invoke("plugin:multiplayer|cmd_join_room", {
        roomCode: options.roomCode,
        playerName: options.playerName,
    })
}

export async function leaveRoom(): Promise<void> {
    await invoke("plugin:multiplayer|cmd_leave_room")
}

export async function getSessionState(): Promise<SessionState> {
    return await invoke("plugin:multiplayer|cmd_get_session_state")
}

export async function queryPeers(): Promise<PeerInfo[]> {
    return await invoke("plugin:multiplayer|cmd_query_peers")
}

export async function recentLogs(limit?: number): Promise<string[]> {
    return await invoke("plugin:multiplayer|cmd_recent_logs", { limit })
}

const ROOM_CODE_CHAR_MAP: Record<string, number> = (() => {
    const map: Record<string, number> = {}
    for (let i = 0; i <= 9; i++) {
        map[String(i)] = i
    }
    for (let i = 0; i < 8; i++) {
        map[String.fromCharCode("A".charCodeAt(0) + i)] = 10 + i
    }
    for (let i = 0; i < 5; i++) {
        map[String.fromCharCode("J".charCodeAt(0) + i)] = 18 + i
    }
    for (let i = 0; i < 11; i++) {
        map[String.fromCharCode("P".charCodeAt(0) + i)] = 23 + i
    }
    return map
})()

export function isRoomCodeValid(input: string): boolean {
    const match =
        /^U\/([0-9A-HJ-NP-Z]{4})-([0-9A-HJ-NP-Z]{4})-([0-9A-HJ-NP-Z]{4})-([0-9A-HJ-NP-Z]{4})$/.exec(
            input,
        )
    if (!match) {
        return false
    }
    const chars = match.slice(1).join("").split("")
    let value = 0n
    let base = 1n
    for (const ch of chars) {
        const mapped = ROOM_CODE_CHAR_MAP[ch]
        if (mapped === undefined) {
            return false
        }
        value += BigInt(mapped) * base
        base *= 34n
    }
    return value % 7n === 0n
}

export async function version(): Promise<string> {
    return await invoke("plugin:multiplayer|cmd_version")
}

export type ConfigureOptions = {
    publicNodes?: string[]
    dataDir?: string
}

export async function configure(options: ConfigureOptions): Promise<void> {
    await invoke("plugin:multiplayer|cmd_configure", {
        publicNodes: options.publicNodes,
        dataDir: options.dataDir,
    })
}
