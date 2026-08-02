// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
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

export async function getAllLevels(instanceId: string): Promise<any> {
    return await invoke("plugin:game-data|cmd_get_all_levels", { instanceId })
}
