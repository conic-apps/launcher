import { Channel, invoke } from "@tauri-apps/api/core"

export enum DownloadTaskType {
    VersionInfo = "VersionInfo",
    Assets = "Assets",
    Libraries = "Libraries",
    MojangJava = "MojangJava",
    AuthlibInjector = "AuthlibInjector",
    ModrinthMod = "ModrinthMod",
    CurseforgeMod = "CurseforgeMod",
    BeatThis = "BeatThis",
    Terracotta = "Terracotta",
    Unknown = "Unknown",
}

export type Checksum = { Sha1: string } | { Sha256: string } | { Sha512: string } | "None"

export type DownloadTaskInfo = {
    url: string
    file: string
    size_bytes?: number
    checksum: Checksum
    task_type: DownloadTaskType
}

export type DownloadState = {
    completedTasks: number
    totalTasks: number
    completedBytes: number
    totalBytes: number
    phase: "VerifyExistingFiles" | "DownloadFiles"
    speed: number
}

export class DownloadTask {
    protected _taskinfo?: DownloadTaskInfo
    protected _taskId?: string
    protected _callbacks?: {
        onProgress?: (progress: DownloadState) => void
    }
    constructor(downloadTask: DownloadTaskInfo, callbacks?: typeof this._callbacks) {
        this._callbacks = callbacks
        this._taskinfo = downloadTask
        this._taskId = crypto.randomUUID()
    }
    async start() {
        const channel = new Channel<DownloadState>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
        }
        await invoke("plugin:download|cmd_spawn_download_task", {
            downloadTask: this._taskinfo,
            taskId: this._taskId,
            channel,
        })
    }
    async cancel() {
        await invoke("plugin:download|cmd_cancel_download_task", { taskId: this._taskId })
    }
}

export function formatBytes(bytes: number): string {
    if (bytes < 1024) {
        return `${bytes} B`
    }

    const units = ["KB", "MB", "GB", "TB"]
    let value = bytes / 1024
    let unitIndex = 0

    while (value >= 1024 && unitIndex < units.length - 1) {
        value /= 1024
        unitIndex++
    }

    let digits = 0

    if (value < 10) {
        digits = 1
    } else if (value < 100) {
        digits = 1
    }

    return `${value.toFixed(digits)} ${units[unitIndex]}`
}
