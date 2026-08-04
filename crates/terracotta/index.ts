// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { DownloadState } from "@conic/download"
import { Channel, invoke } from "@tauri-apps/api/core"

export enum TerracottaErrorKind {
    Io = "Io",
    ToStr = "ToStr",
    Network = "Network",
    AllSourceFailed = "AllSourceFailed",
    LibLoader = "LibLoader",
    ChecksumMismatch = "ChecksumMismatch",
    Aborted = "Aborted",
    Nul = "Nul",
    NotLoaded = "NotLoaded",
    NoContext = "NoContext",
    ContextAlreadyExists = "ContextAlreadyExists",
    TerraResult = "TerraResult",
}

/** Result codes returned by the terracotta C API (see terracotta.h). */
export enum TerracottaResult {
    Ok = "Ok",
    InvalidHandle = "InvalidHandle",
    InvalidArgument = "InvalidArgument",
    BadState = "BadState",
    InvalidRoomCode = "InvalidRoomCode",
    AlreadyActive = "AlreadyActive",
    Internal = "Internal",
    OutOfMemory = "OutOfMemory",
    NoEvent = "NoEvent",
    ShuttingDown = "ShuttingDown",
}

/** Session state ids (mirrors Terracotta's AppState). */
export enum TerracottaStateId {
    Waiting = 0,
    HostScanning = 1,
    HostStarting = 2,
    HostOk = 3,
    GuestConnecting = 4,
    GuestStarting = 5,
    GuestOk = 6,
    Exception = 7,
}

/** Incremental events polled with `pollEvent()`. */
export enum TerracottaEventType {
    StateChanged = 1,
    PlayerJoined = 2,
    PlayerLeft = 3,
    ConnectionDifficulty = 4,
    HostReady = 5,
    GuestReady = 6,
    Error = 7,
}

export type TerracottaConfig = {
    /** Extra EasyTier public nodes; the library always appends built-in defaults. */
    publicNodes?: string[]
    /** Persistent files (EasyTier binary, machine-id). Defaults to a temp dir. */
    dataDir?: string
    /** MOTD used to identify Terracotta virtual servers in LAN broadcasts. */
    motd?: string
}

/**
 * Full state snapshot.
 * `detail` is state-specific JSON:
 *  - `HostOk`:    `{ "port": N, "profiles": [...] }`
 *  - `GuestOk`:   `{ "url": "127.0.0.1:port", "profiles": [...] }`
 *  - `Exception`: `{ "error": { "code": N, "message": "..." } }`
 */
export type TerracottaState = {
    version: number
    state: TerracottaStateId
    roomCode: string
    detail: Record<string, unknown>
}

/** An incremental event polled with `pollEvent()`. */
export type TerracottaEvent = {
    sequence: number
    type: TerracottaEventType
    payload: Record<string, unknown>
}

export class TerracottaLibraryDownloadTask {
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
        await invoke("plugin:terracotta|cmd_spawn_download_library_task", {
            channel,
        })
    }
    async cancel() {
        await invoke("plugin:terracotta|cmd_cancel_download_library_task")
    }
}

export async function isLibraryValid(): Promise<boolean> {
    try {
        await invoke("plugin:terracotta|cmd_check_library_valid")
        return true
    } catch {
        return false
    }
}

/**
 * Loads the terracotta library once. The library stays loaded for the whole
 * launcher run; only the context handle is created/destroyed per session.
 */
export async function ensureLibrary(): Promise<void> {
    await invoke("plugin:terracotta|cmd_ensure_library")
}

/** Creates a context and applies `config`. Must be called after `ensureLibrary()`. */
export async function createContext(config?: TerracottaConfig): Promise<void> {
    await invoke("plugin:terracotta|cmd_create_context", { config: config ?? {} })
}

/** Destroys the active context, killing EasyTier / the fake server. */
export async function destroyContext(): Promise<void> {
    await invoke("plugin:terracotta|cmd_destroy_context")
}

/**
 * Hosts a room. `roomCode == null` generates a new one; otherwise it must be a
 * valid "U/XXXX-XXXX-XXXX-XXXX" code. `playerName == null` uses the default
 * anonymous name.
 */
export async function createRoom(playerName?: string, roomCode?: string): Promise<void> {
    await invoke("plugin:terracotta|cmd_create_room", { playerName, roomCode })
}

/** Joins an existing room. `roomCode` is required and validated. */
export async function joinRoom(roomCode: string, playerName?: string): Promise<void> {
    await invoke("plugin:terracotta|cmd_join_room", { roomCode, playerName })
}

/** Aborts any active session and returns to the `Waiting` state. */
export async function setWaiting(): Promise<void> {
    await invoke("plugin:terracotta|cmd_set_waiting")
}

/** Returns a full state snapshot. */
export async function getState(): Promise<TerracottaState> {
    return await invoke("plugin:terracotta|cmd_get_state")
}

/** Pops the next pending event, or `null` when the queue is empty. Non-blocking. */
export async function pollEvent(): Promise<TerracottaEvent | null> {
    return await invoke("plugin:terracotta|cmd_poll_event")
}

/** Verifies a room code without needing an active context. */
export async function verifyRoomCode(roomCode: string): Promise<boolean> {
    return await invoke("plugin:terracotta|cmd_verify_room_code", { roomCode })
}

/** Returns the loaded library version string (e.g. "0.1.0"). */
export async function getVersion(): Promise<string> {
    return await invoke("plugin:terracotta|cmd_version")
}
