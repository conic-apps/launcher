// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config } from "@conic/config"
import { Instance } from "@conic/instance"
import { Channel, invoke } from "@tauri-apps/api/core"

export type VersionManifest = {
    latest: {
        release: string
        snapshot: string
    }
    versions: {
        id: string
        type: string
        url: string
        time: string
        releaseTime: string
        sha1: string
        complianceLevel: number
    }[]
}

export async function getMinecrafVersionManifest(): Promise<VersionManifest> {
    return await invoke("plugin:install|cmd_get_minecraft_version_list")
}

type FabricArtifactVersion = {
    game_version?: string
    separator?: string
    build?: number
    maven: string
    version: string
    stable: boolean
}

export type FabricLoaderArtifact = {
    loader: FabricArtifactVersion
    intermediary: FabricArtifactVersion
    launcher_meta: {
        version: number
        libraries: {
            client: { name?: string; url?: string }[]
            common: { name?: string; url?: string }[]
            server: { name?: string; url?: string }[]
        }
        main_class: NonNullable<object>
    }
}

export async function getFabricVersionList(mcversion: string): Promise<FabricLoaderArtifact[]> {
    return await invoke("plugin:install|cmd_get_fabric_version_list", { mcversion })
}

export type QuiltVersion = {
    loader: {
        separator: string
        build: number
        maven: string
        version: string
    }
    hashed: {
        maven: string
        version: string
    }
    intermediary: {
        maven: string
        version: string
    }
    launcher_meta: {
        version: number
        libraries: {
            client: { name: string; url: string }[]
            common: { name: string; url: string }[]
            server: { name: string; url: string }[]
        }
        main_class: {
            client?: string
            server?: string
            server_launcher?: string
        }
    }
}

export async function getQuiltVersionList(mcversion: string): Promise<QuiltVersion[]> {
    return await invoke("plugin:install|cmd_get_quilt_version_list", { mcversion })
}
export type ForgeVersionItem = {
    _id: string
    build: number
    __v: number
    version: string
    modified: string
    mcversion: string
    files: {
        format: string
        category: string
        hash?: string
    }[]
    branch: object
}

export async function getForgeVersionList(mcversion: string): Promise<ForgeVersionItem[]> {
    return await invoke("plugin:install|cmd_get_forge_version_list", { mcversion })
}

export async function getNeoforgedVersionList(mcversion: string): Promise<string[]> {
    return await invoke("plugin:install|cmd_get_neoforged_version_list", { mcversion })
}

export enum Job {
    Prepare = "Prepare",
    InstallGame = "InstallGame",
    InstallJava = "InstallJava",
    InstallModLoader = "InstallModLoader",
}

export enum DownloaderStep {
    VerifyExistingFiles = "VerifyExistingFiles",
    DownloadFiles = "DownloadFiles",
}

export enum InstallErrorKind {
    Io = "Io",
    Network = "Network",
    InstanceBroken = "InstanceBroken",
    InvalidForgeVersion = "InvalidForgeVersion",
    ForgeInstallerFailed = "ForgeInstallerFailed",
    NeoforgedInstallerFailed = "NeoforgedInstallerFailed",
    HttpResponseNotSuccess = "HttpResponseNotSuccess",
    InvalidVersionJson = "InvalidVersionJson",
    VersionMetadataNotfound = "VersionMetadataNotfound",
    JsonParse = "JsonParse",
    ResolveVersionJsonFailed = "ResolveVersionJsonFailed",
    Sha1Missmatch = "Sha1Missmatch",
    UrlParse = "UrlParse",
    NoSupportedJavaRuntim = "NoSupportedJavaRuntime",
    Aborted = "Aborted",
}

export type Progress = {
    id: `${string}-${string}-${string}-${string}-${string}`
    completed: number
    total: number
    speed: number
    job: Job
    downloaderStep: DownloaderStep
}

export type Callbacks = {
    onStart?: (id: string) => void
    onProgress?: (task: Progress) => void
    onFailed?: (id: string, error: { kind: InstallErrorKind; message: string }) => void
    onSucceed?: (id: string) => void
    onPaused?: (id: string) => void
    onResumed?: (id: string) => void
    onCancelled?: (id: string) => void
}

type InstallEvent = {
    completed: number
    total: number
    downloader_step: DownloaderStep
    speed: number
    job: Job
}

export class InstallTask {
    private _promise?: Promise<void>
    private _config: Config
    private _instance: Instance
    completed: number
    total: number
    downloader_step: DownloaderStep
    speed: number
    job: Job
    constructor(config: Config, instance: Instance) {
        ;((this._config = config), (this._instance = instance))
        this.completed = 0
        this.total = 0
        this.downloader_step = DownloaderStep.DownloadFiles
        this.speed = 0
        this.job = Job.Prepare
    }
    start(callbacks?: Callbacks) {
        this._promise = installPromise(this._config, this._instance, callbacks)
    }
    cancel() {
        invoke("plugin:install|cmd_cancel_install_task")
    }
}

async function installPromise(config: Config, instance: Instance, callbacks?: Callbacks) {
    const channel = new Channel<InstallEvent>()
    const id = crypto.randomUUID()
    channel.onmessage = (message) => {
        callbacks?.onProgress?.({
            id,
            completed: message.completed,
            total: message.total,
            speed: message.speed,
            job: message.job,
            downloaderStep: message.downloader_step,
        })
    }
    try {
        await invoke("plugin:install|cmd_create_install_task", { config, instance, channel })
        callbacks?.onSucceed?.(id)
    } catch (error: any) {
        if (error.kind && error.message) {
            const kind = error.kind as InstallErrorKind
            if (kind === InstallErrorKind.Aborted) {
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
