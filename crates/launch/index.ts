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

type Progress = {
    job: Job
    progress?: {
        completed: number
        total: number
        step: "VerifyExistingFiles" | "DownloadFiles"
        speed: number
    }
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

export type Callbacks = {
    onStart?: (id: string) => void
    onProgress?: (task: Progress) => void
    onFailed?: (id: string, error: { kind: LaunchErrorKind; message: string }) => void
    onSucceed?: (id: string) => void
    onPaused?: (id: string) => void
    onResumed?: (id: string) => void
    onCancelled?: (id: string) => void
}

export class LaunchTask {
    private _promise?: Promise<void>
    private _config: Config
    private _instance: Instance
    progress?: {}
    job: Job
    constructor(config: Config, instance: Instance) {
        this._config = config
        this._instance = instance
        this.job = Job.Prepare
    }
    start(callbacks?: Callbacks) {
        this._promise = launchPromise(this._config, this._instance, callbacks)
    }
    async startAndWait(callbacks?: Callbacks) {
        this._promise = launchPromise(this._config, this._instance, callbacks)
        await this._promise
    }
    cancel() {
        invoke("plugin:install|cmd_cancel_install_task")
    }
}

async function launchPromise(config: Config, instance: Instance, callbacks?: Callbacks) {
    const channel = new Channel<Progress>()
    const id = crypto.randomUUID()
    channel.onmessage = (message) => {
        callbacks?.onProgress?.(message)
    }
    try {
        await invoke("plugin:install|cmd_create_install_task", { config, instance, channel })
        callbacks?.onSucceed?.(id)
    } catch (error: any) {
        if (error.kind && error.message) {
            const kind = error.kind as LaunchErrorKind
            if (kind === LaunchErrorKind.Aborted) {
                callbacks?.onCancelled?.(id)
            } else {
                callbacks?.onFailed?.(id, {
                    kind,
                    message: error.message as string,
                })
            }
        } else {
            throw error
        }
    }
}
