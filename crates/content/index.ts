// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export enum IntBoolean {
    True = 1,
    False = 0,
}

export type FastNbtIntArray = {
    __fastnbt_int_array: number[]
}

export type Level = {
    Data: {
        difficulty_settings?: {
            hardcore?: IntBoolean
            locked?: IntBoolean
            difficulty?: "peaceful" | "easy" | "normal" | "hard" | string
        }
        Version?: {
            Series?: "main" | string
            Snapshot?: IntBoolean
            Name?: string
            Id?: number
        }
        allowCommands?: IntBoolean
        spawn?: {
            yaw?: number
            pos?: FastNbtIntArray
            dimension?: string
            pitch?: number
        }
        DataVersion?: number
        DataPacks?: {
            Disabled?: string[]
            Enabled?: string[]
        }
        singleplayer_uuid?: FastNbtIntArray
        Time?: number
        version?: number
        WasModded?: IntBoolean
        GameType?: number
        LastPlayed?: number
        ServerBrands?: string[]
        initialized?: IntBoolean
        LevelName?: string
        [key: string]: unknown
    }
}

export type Resourcepack = {
    metadata: {
        pack: {
            description: string
            min_format: number[]
            max_format: number[]
        }
    }
    icon?: string
    name: string
}

export async function getAllLevels(instanceId: string): Promise<Record<string, Level>> {
    return await invoke("plugin:content|cmd_get_all_levels", { instanceId })
}

export async function getSaveIcon(instanceId: string, folderName: string): Promise<string> {
    return await invoke("plugin:content|cmd_get_save_icon", { instanceId, folderName })
}

export async function getAllResourcepacks(instanceId: string): Promise<Resourcepack[]> {
    return await invoke("plugin:content|cmd_get_all_resourcepacks", { instanceId })
}

export type WorldMapRenderRequest = {
    instanceId: string
    folderName: string
    width: number
    height: number
    centerX?: number
    centerZ?: number
    dimension?: string
    water?: boolean
    shading?: boolean
    altitudeShading?: boolean
}

export type WorldMapRenderResult = {
    width: number
    height: number
    pixels: number[]
}

export async function renderWorldMap(
    request: WorldMapRenderRequest,
): Promise<WorldMapRenderResult> {
    return await invoke("plugin:content|cmd_render_world_map", { request })
}
