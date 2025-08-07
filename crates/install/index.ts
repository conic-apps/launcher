// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config } from "@conic/config"
import { Instance } from "@conic/instance"
import { invoke } from "@tauri-apps/api/core"

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
    return await invoke("plugin:config|cmd_get_minecraft_version_list")
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
    return await invoke("plugin:config|cmd_get_fabric_version_list", { mcversion })
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
    return await invoke("plugin:config|cmd_get_quilt_version_list", { mcversion })
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
    return await invoke("plugin:config|cmd_get_forge_version_list", { mcversion })
}

export async function getNeoforgedVersionList(mcversion: string): Promise<string[]> {
    return await invoke("plugin:config|cmd_get_neoforged_version_list", { mcversion })
}

export async function install(config: Config, instance: Instance) {
    return await invoke("plugin:config|cmd_install", { config, instance })
}
