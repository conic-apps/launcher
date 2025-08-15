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

export enum InstallErrorKind {
    AlreadyInstalling = "AlreadyInstalling",
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
    NoSupportedJavaRuntime = "NoSupportedJavaRuntime",
    Aborted = "Aborted",
}

export enum Job {
    Prepare = "Prepare",
    InstallGame = "InstallGame",
    InstallJava = "InstallJava",
    InstallModLoader = "InstallModLoader",
}

export type InstallProgress = {
    job: Job
    progress?: {
        completed: number
        total: number
        step: "VerifyExistingFiles" | "DownloadFiles"
        speed: number
    }
}

export class InstallTask {
    protected _config: Config
    protected _instance: Instance
    job: Job
    progress?: InstallProgress
    protected _callbacks?: {
        onStart?: () => void
        onProgress?: (task: InstallProgress) => void
        onFailed?: (error: { kind: InstallErrorKind; message: string }) => void
        onSucceed?: () => void
        onCancelled?: () => void
    }
    constructor(config: Config, instance: Instance, callbacks?: typeof this._callbacks) {
        this._config = config
        this._instance = instance
        this.job = Job.Prepare
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<InstallProgress>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
        }
        try {
            await invoke("plugin:install|cmd_create_install_task", {
                config: this._config,
                instance: this._instance,
                channel,
            })
            this._callbacks?.onSucceed?.()
        } catch (error: any) {
            if (error.kind && error.message) {
                const kind = error.kind as InstallErrorKind
                if (kind === InstallErrorKind.Aborted) {
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
        await invoke("plugin:install|cmd_cancel_install_task")
    }
}
