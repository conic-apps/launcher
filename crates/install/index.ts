// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
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

export async function getForgeVersionList(mcversion: string): Promise<Record<string, string[]>> {
    return await invoke("plugin:install|cmd_get_forge_version_list", { mcversion })
}

export async function getNeoforgeVersionList(mcversion: string): Promise<string[]> {
    return await invoke("plugin:install|cmd_get_neoforge_version_list", { mcversion })
}

export enum InstallErrorKind {
    AlreadyInstalling = "AlreadyInstalling",
    Io = "Io",
    Network = "Network",
    InstanceBroken = "InstanceBroken",
    InvalidForgeVersion = "InvalidForgeVersion",
    ForgeInstallerFailed = "ForgeInstallerFailed",
    NeoforgeInstallerFailed = "NeoforgeInstallerFailed",
    InvalidVersionJson = "InvalidVersionJson",
    VersionMetadataNotfound = "VersionMetadataNotfound",
    JsonParse = "JsonParse",
    ResolveVersionJsonFailed = "ResolveVersionJsonFailed",
    ChecksumMissmatch = "ChecksumMissmatch",
    UrlParse = "UrlParse",
    NoSupportedJavaRuntime = "NoSupportedJavaRuntime",
    Aborted = "Aborted",
    Zip = "Zip",
    NoAvailableForgeVersion = "NoAvailableForgeVersion ",
    InvalidAuthlibResponse = "InvalidAuthlibResponse",
    ChunkLengthMismatch = "ChunkLengthMismatch",
}

export enum Job {
    Prepare = "Prepare",
    InstallGame = "InstallGame",
    InstallJava = "InstallJava",
    InstallModLoader = "InstallModLoader",
}

export type InstallProgress =
    | {
          job: Job.Prepare
      }
    | {
          job: Job.InstallGame
          downloadState?: {
              completed: number
              total: number
              phase: "VerifyExistingFiles" | "DownloadFiles"
              speed: number
          }
      }
    | {
          job: Job.InstallJava
          downloadState?: {
              completed: number
              total: number
              phase: "VerifyExistingFiles" | "DownloadFiles"
              speed: number
          }
      }
    | {
          job: Job.InstallModLoader
      }

export class InstallTask {
    protected _config: Config
    protected _instance: Instance
    protected _callbacks?: {
        onProgress?: (progress: InstallProgress) => void
    }
    constructor(config: Config, instance: Instance, callbacks?: typeof this._callbacks) {
        this._config = config
        this._instance = instance
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<InstallProgress>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
        }
        await invoke("plugin:install|cmd_spawn_install_task", {
            config: this._config,
            instance: this._instance,
            channel,
        })
    }
    async cancel() {
        await invoke("plugin:install|cmd_cancel_install_task")
    }
}
