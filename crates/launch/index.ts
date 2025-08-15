// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config } from "@conic/config"
import { Instance } from "@conic/instance"
import { Channel, invoke } from "@tauri-apps/api/core"

enum Job {
    Prepare = "Prepare",
    RefreshAccount = "RefreshAccount",
    CompleteFiles = "CompleteFiles",
    GenerateScriptlet = "GenerateScriptlet",
    WaitForLaunch = "WaitForLaunch",
    LogSettingUser = "LogSettingUser",
    LogLwjglVersion = "LogLwjglVersion",
    LogOpenALLoaded = "LogOpenALLoaded",
    LogTextureLoaded = "LogTextureLoaded",
}

enum LaunchErrorKind {
    AlreadyInLaunching = "AlreadyInLaunching",
    Io = "Io",
    VersionJsonParse = "VersionJsonParse",
    InvalidVersionJson = "InvalidVersionJson",
    InvalidMinecraftVersion = "InvalidMinecraftVersion",
    InvalidInstance = "InvalidInstance",
    DecompressionFailed = "DecompressionFailed",
    Network = "Network",
    TakeMinecraftStdoutFailed = "TakeMinecraftStdoutFailed",
    AccountError = "AccountError",
    Aborted = "Aborted",
    Sha1Missmatch = "Sha1Missmatch",
    HttpResponseNotSuccess = "HttpResponseNotSuccess",
    Other = "Other",
}

type LaunchProgress = {
    job: Job
    progress?: {
        completed: number
        total: number
        step: "VerifyExistingFiles" | "DownloadFiles"
        speed: number
    }
}

/**
 * Usage:
 * ```ts
 * const task = new LaunchTask;
 * task.callbacks = {
 *   onProgress: ...
 * }
 * await task.start()
 * ```
 */
export class LaunchTask {
    private _config: Config
    private _instance: Instance
    private _callbacks?: {
        onStart?: () => void
        onProgress?: (task: LaunchProgress) => void
        onFailed?: (error: { kind: LaunchErrorKind; message: string }) => void
        onSucceed?: () => void
        onCancelled?: () => void
    }
    progress?: {}
    job: Job
    constructor(config: Config, instance: Instance, callbacks?: typeof this._callbacks) {
        this._config = config
        this._instance = instance
        this.job = Job.Prepare
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<LaunchProgress>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
            this.progress = message.progress
            this.job = message.job
        }
        try {
            await invoke("plugin:launch|cmd_create_launch_task", {
                config: this._config,
                instance: this._instance,
                channel,
            })
            this._callbacks?.onSucceed?.()
        } catch (error: any) {
            if (error.kind && error.message) {
                const kind = error.kind as LaunchErrorKind
                if (kind === LaunchErrorKind.Aborted) {
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
        await invoke("plugin:launch|cmd_cancel_launch_task")
    }
}
