// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Channel, invoke } from "@tauri-apps/api/core"

type BeatProgress = {
    completed: number
    total: number
    step: "VerifyExistingFiles" | "DownloadFiles" | "VerifyResult"
    speed: number
}

export type BeatAnalysis = {
    beats: number[]
    downbeats: number[]
    mel: {
        shape: number[]
        data: number[]
    }
    beatLogits: number[]
    downbeatLogits: number[]
}

export enum BeatErrorKind {
    Io = "Io",
    ToStr = "ToStr",
    Network = "Network",
    AllSourceFailed = "AllSourceFailed",
    LibLoader = "LibLoader",
    ChecksumMismatch = "ChecksumMismatch",
    Aborted = "Aborted",
}

export class BeatLibraryDownloadTask {
    private _callbacks?: {
        onStart?: () => void
        onProgress?: (progress: BeatProgress) => void
        onFailed?: (error: { kind: BeatErrorKind; message: string }) => void
        onSucceed?: () => void
        onCancelled?: () => void
    }
    constructor(callbacks?: typeof this._callbacks) {
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<BeatProgress>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
        }
        try {
            this._callbacks?.onStart?.()
            await invoke("plugin:beat|cmd_spawn_download_library_task", {
                channel,
            })
            this._callbacks?.onSucceed?.()
        } catch (error: any) {
            if (error.kind && error.message) {
                const kind = error.kind as BeatErrorKind
                if (kind === BeatErrorKind.Aborted) {
                    this._callbacks?.onCancelled?.()
                } else {
                    this._callbacks?.onFailed?.(error)
                }
            } else {
                throw error
            }
        }
    }
    async cancel() {
        await invoke("plugin:beat|cmd_cancel_download_library_task")
    }
}

export async function parseAudioFile(path: string): Promise<BeatAnalysis> {
    return await invoke("plugin:beat|cmd_parse_audio_file", { path })
}
