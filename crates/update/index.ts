// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { UpdateChannel } from "@conic/config"
import { Channel, invoke } from "@tauri-apps/api/core"

export type UpdateInfo = {
    version: string
    date?: number
    notes?: string
    downloadUrl: string
}

export type UpdateProgress =
    | { phase: "checking" }
    | { phase: "downloading"; downloaded: number; total?: number }
    | { phase: "downloaded" }
    | { phase: "installing" }

export async function checkUpdate(channel: UpdateChannel): Promise<UpdateInfo | null> {
    return await invoke("plugin:update|cmd_check_update", { channel })
}

export async function downloadAndInstallUpdate(
    channel: UpdateChannel,
    onProgress: (progress: UpdateProgress) => void,
): Promise<void> {
    const progressChannel = new Channel<UpdateProgress>()
    progressChannel.onmessage = onProgress
    await invoke("plugin:update|cmd_download_and_install_update", {
        channel,
        onProgress: progressChannel,
    })
}

export async function cancelUpdate(): Promise<void> {
    await invoke("plugin:update|cmd_cancel_update")
}
