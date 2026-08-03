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

export async function checkLibraryValid(): Promise<void> {
    return await invoke("plugin:terracotta|cmd_check_library_valid")
}
